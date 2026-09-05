
use crate::structures::state::Node_Min_Max;
pub fn min_max(mut alpha:f64,mut beta:f64,role:i8,curr:&mut Node_Min_Max,depth:usize)->f64
{
	if(depth==0)
	{
		// i will have a method to which will take the current node and evaluate the
		// board then attacch the value to the node and return the value of the node
		return curr.evaluate();
	}
	else if(curr.children.len()==0)
	{
		curr.calculate_children();
	}
	// iterate over all the children of the current node
	for i in 0..curr.children.len()
	{
		let mut child=&mut curr.children[i];
		if role==1
		{
			alpha=alpha.max(min_max(alpha,beta,-role,child,depth-1));
		}
		else
		{
			beta=beta.min(min_max(alpha,beta,-role,child,depth-1));
		}
		if beta<=alpha
		{
			break;
		}
	}
	return if role==1 {alpha} else {beta};
}