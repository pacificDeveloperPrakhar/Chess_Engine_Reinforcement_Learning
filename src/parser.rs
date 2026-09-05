use crate::structures::annotations::*;
pub fn bitboards_to_modified_epd(bitboards:[[u64;7];2])->Vec<char>
{
  let mut result=String::from("0000000000000000000000000000000000000000000000000000000000000000");
  let mut result: Vec<char> = result.chars().collect();
  {
    let mut itera=0;
    for i in 0..2
    {
      for j in 0..6
      {
        let board=bitboards[i][j];
        while(itera<64)
        {
        if (board&(1<<itera))!=0
        {
          result[itera]= Piece::as_char(j,i as usize);
        }
         itera+=1;
        }
        itera=0;
      }
    }
  }
 for i in 0..8
 {
  result.insert(i*8+8+i,'/');
 }
 return result;
}

pub fn bitboards_to_fen(bitboards:[[u64;7];2])->String
{
  return String::from("hello");
}