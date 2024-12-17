use tokenizer::*;

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
