pub fn min_max(alpha:f64,beta:f64,role:i8,curr:Node)->f64
{
	if(curr.children.len()==0)
	{
		// i will have a method to which will take the current node and evaluate the
		// board then attacch the value to the node and return the value of the node
		return curr.bitboards.evaluate();
	}
	// iterate over all the children of the current node
	for i in 0..curr.children.len()
	{
		let child=curr.children[i];
		if role==1
		{
			alpha=Math::max(alpha,min_max(alpha,beta,-role,child));
		}
		else
		{
			beta=Math::min(beta,min_max(alpha,beta,-role,child));
		}
		if beta<=alpha
		{
			break;
		}
	}
	return if role==1 {alpha} else {beta};
}