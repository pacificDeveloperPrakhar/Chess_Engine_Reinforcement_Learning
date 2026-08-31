pub mod state;
pub mod annotations;

pub struct Node_Min_Max
{
	pub bitboards:state::BitBoards,
	pub children:Vec<Node_Min_Max>,
	pub value:f64,
}

impl Node_Min_Max
{
	pub fn new_state(bitboards:state::BitBoards,row,column)->Node_Min_Max
	{
		Node_Min_Max{bitboards:bitboards,children:Vec::new(),value:0.0}
	}
}