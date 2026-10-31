use der::asn1::{
	Any,BitString,GeneralizedTime,Int,ObjectIdentifier,OctetString,SetOfVec,UtcTime,
}
// Any->data will be encoded on the basis of the context
// BitString-> this will hold the bits in continuous manner
//Generalized Time ->we are converting the UtcTime to this
//Int-> when we have a very big number asn requires to make 2s complement and big endian format
//ObjectIdentifier -> this is to create a globally unqiue identifier like 1.2.840.113549.1.1.11,
//SetOfVec -> set of unordered same type (tlv) sorted ,this is how der requries them to be

use der::{Choice,Enumerated,Sequence,ValueOrd}
// ValueOrd->how to put things in order,in what order
//Sequence tell how to place the field of struct after parsing ,squence keep everyhting serial ,one after another
//Choice ->tell the value could be either of two
//Enumerated ->tell the option could be any of that,with number u can assign it to them
// ValueOrd->define  how to keep the value in the asn ,like how to sort them while packing

/// Version ::= INTEGER { v1(0), v2(1), v3(2) }
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Enumerated)]
#[asn1(type = "INTEGER")]
#[repr(u8)]
pub enum Version {
    #[default]
    V1 = 0,
    V2 = 1,
    V3 = 2,
}


/// AlgorithmIdentifier ::= SEQUENCE {
///     algorithm   OBJECT IDENTIFIER,
///     parameters  ANY DEFINED BY algorithm OPTIONAL
/// }
///
/// For your cert the algorithm OID is 1.2.840.10045.4.3.2
/// (ecdsa-with-SHA256) and `parameters` is absent.
#[derive(Clone, Debug, Eq, PartialEq, Sequence, ValueOrd)]
pub struct AlgorithmIdentifier {
    pub algorithm: ObjectIdentifier,
    pub parameters: Option<Any>,
}

#[derive(Clone,Copy,Debug,Eq,PartialEq,Sequence,ValueOrd)]
pub struct field_type_and_value{
	pub oid:ObjectIdentifier,
	pub value:Any
}


// set of attribute to define 
// this is used because the der requires the set ordered element and not sequence
// suppose if i have collection of sst,sd being sent over for a ue , then if i require to send them over
// the asn i need to sort them and then send them

pub type set_of_attribute =SetOfVec<field_type_and_value>;

// now there is something called rdn sequence so it seems that is just different collection of values of same type
/// Name ::= RDNSequence ::= SEQUENCE OF set_of_attribute
pub type rdn_sequence=Vec<set_of_attribute>;

// now to define the time field there gonna be of two types 
#[drive(Copy,Clone,Debug,PartialEq,Eq,Choice,ValueOrd)]
#[repr(u8)]
pub enum Time
{
    #[asn1(type = "UTCTime")]
    UtcTime(UtcTime),
    #[asn1(type = "GeneralizedTime")]
    GeneralTime(GeneralizedTime),
}


/// Validity ::= SEQUENCE { notBefore Time, notAfter Time }
///
/// The 14-day window lives here. This is what the browser checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Sequence, ValueOrd)]
pub struct Validity {
    pub not_before: Time,
    pub not_after: Time,
}

// adding the struct to define what the public key need to be which is suppose to be 
// embedded into the certificate

#[derive(Clone, Debug, Eq, PartialEq, Sequence, ValueOrd)]
pub struct SubjectPublicKeyInfo {
    pub algorithm: AlgorithmIdentifier,
    pub subject_public_key: BitString,
}
// this is just to add somthing on top of the exsiting certificate 
// early provision for the certificate implementation did not have the room for it
#[derive(Clone,Debug,Eq,PartialEq,Sequence,ValueOrd)]
pub struct Extension
{
    // id for the extension ,requires to be unique
    pub exten_id:ObjectIdentifier,
    #[asn1(default="bool::default")]
    pub critical:bool,
    // value of the extension info
    pub extn_value:OctetString
}

#[derive(Copy,Clone,Eq,PartialEq,Sequence,ValueOrd)]
pub struct tbsCertificate
{
    #[asn1(context_specific = "0",default = "Default::default")]
    pub version:Version,
    pub serial_number: Int,
    // will use the same struct that we are using for the public key
    pub signature:AlgorithmIdentifier,
    pub issuer:rdn_sequence,
    pub validity:Validity,
    pub subject:rdn_sequence,
     pub subject_public_key_info: SubjectPublicKeyInfo,

    #[asn1(context_specific = "1", tag_mode = "IMPLICIT", optional = "true")]
    pub issuer_unique_id: Option<BitString>,

    #[asn1(context_specific = "2", tag_mode = "IMPLICIT", optional = "true")]
    pub subject_unique_id: Option<BitString>,

    /// Your SANs live in here, under OID 2.5.29.17.
    #[asn1(context_specific = "3", tag_mode = "EXPLICIT", optional = "true")]
    pub extensions: Option<Extensions>,
}

// certificate that needs to be signed and send to browser for pinning certificate
#[derive(Debug,Clone,Eq,PartialEq,Sequence)]
pub struct Certificate
{
    pub tbs_certificate:tbsCertificate,
    pub signature_algorithm:AlgorithmIdentifier,
    pub signature:BitString
}