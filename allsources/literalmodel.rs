use tokenizer::*;
use std::fmt;
/****************************************
****	Literal Statement
*****************************************/

pub struct LiteralStatement{
	// cli : & 'a CLI<'a>,
	pub literal_id : Token,
	pub literal_value : Vec<Token>,
}

impl LiteralStatement{
	pub fn new(literal_id:Token, literal_value : Vec<Token>) -> LiteralStatement{
		LiteralStatement{
			//cli : cli,
			literal_id : literal_id,
			literal_value : literal_value.clone(),
		}
	}
}

impl fmt::Display for LiteralStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Literal: {} = {}",self.literal_id.token_value,token_list_text(&self.literal_value))
	}
}
