use tokenizer::*;
use std::fmt;

/****************************************
****	Struct
*****************************************/
#[derive(PartialEq, Clone, Eq)]
pub struct Struct{
	pub name : String,
	pub members : Vec<StructMember>,
}

impl Struct{
	pub fn new(name : String) -> Struct{
		Struct{
			name : name,
			members : Vec::new(),
		}
	}

	pub fn print(&self){
		for m in &self.members{
			print!("     {}", m.name);
			if m.initializer.len() > 0{
				print!(" ={}",token_list_text(&m.initializer));
			}
			print!(";\n");
		}
	}
}

/****************************************
****	StructMember
*****************************************/
#[derive(PartialEq, Clone, Eq)]
pub struct StructMember{
	pub name : String,
	pub initializer : Vec<Token>,
}

impl StructMember{
	pub fn new(name : String, initializer : &Vec<Token>) -> StructMember{
		StructMember{
			name : name,
			initializer : initializer.clone(),
		}
	}
}

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
