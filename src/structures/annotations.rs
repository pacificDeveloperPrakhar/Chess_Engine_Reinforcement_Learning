
use std::convert::From;

#[derive(Debug,Clone,Copy)]
#[repr(usize)]
pub enum PieceValue
{
    P=1,
    N=5,
    B=6,
    R=4,
    Q=16,
    K=255
}
impl From<usize> for PieceValue
{
   fn from(c:usize)->Self
    {
        match c
        {
             0=>PieceValue::P,
             1=>PieceValue::N,
             2=>PieceValue::B,
             3=>PieceValue::R,
             4=>PieceValue::K,
             5=>PieceValue::Q,
            _=> PieceValue::P
        }
    }
}


// this denote the piece type and the color of the piece, for example white knight or black queen etc
#[derive(Debug,Clone,Copy)]
#[repr(usize)]
pub enum Piece
{
     P=0, //pawns
     N=1,//knight
     B=2,//bishop
     R=3,//rook
     K=4,//king
     Q=5,//queen
     A=6,//all the pieces of one color,for example all the white pieces or all the black pieces

    }
    // enum corresponding to the color of the pieces
#[derive(Debug,Clone,Copy)]
#[repr(usize)]
pub enum PieceColor
{
    W=0,
    B=1
}
// implementing the From trait for the piece enum which has a function called from takes an input of one data type and then return
// the data type for what we have specified for 
impl From<char> for Piece{
    fn from(c:char)->Self
    {
     match c{
        'Q'|'q'=>Piece::Q, 
        'K'|'k'=>Piece::K,
        'B'|'b'=>Piece::B,
        'R'|'r'=>Piece::R,
        'P'|'p'=>Piece::P,
        'N'|'n'=>Piece::N,
        _=> Piece::A
    }
}
} 

impl Piece {
    pub fn as_char(piece:usize, side: usize) -> char {
        let c = match piece {
            0 => 'p',
            1 => 'n',
            2 => 'b',
            3 => 'r',
            4 => 'k',
            5 => 'q',
            _=>'p'
        };

        if side == 1 {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }
}


impl From<char> for PieceColor
{
    fn from(c:char)->Self
    {
        match c
        {
            'W'|'w'=>PieceColor::W,
            'B'|'b'=>PieceColor::B,
            _=>PieceColor::W
        }
    }
}

pub enum AttackType
{
    diagnol,
    horizontal_vertical=1,
    pawn,
    square,
    l_shape
}