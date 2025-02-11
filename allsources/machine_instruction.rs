use std::fmt;
use tokenizer::*;
use opcode::*;
use macrolib::*;
use regex::Regex;


/****************************************
****	Machine Instruction
*****************************************/
#[derive(PartialEq, Clone)]
pub struct MachineInstruction{
	pub opcode : Opcode,
	pub opcode_mode : OpcodeMode,
	pub function_num : usize,		// function number
	pub block_num : usize,			// block number for break/continue
	pub address : usize,			// absolute or relative address, parameter count for functon calls 
	pub qualifier : Vec<usize>,		// argument count (1 element) Array index, 1 element, target block for bl
	pub interner : usize,			// The name associated with this instruction (not all instructions have one)
	pub literal : Token,
}

impl MachineInstruction{
	pub fn new(opcode : Opcode, opcode_mode : OpcodeMode, function_num : usize, block_num : usize, adr : usize, qualifier : Vec<usize>, interner : usize, literal : Token) -> MachineInstruction{
		MachineInstruction{
			opcode : opcode,
			opcode_mode : opcode_mode,
			function_num : function_num,
			block_num : block_num,
			address: adr,
			qualifier : qualifier,
			interner : interner,
			literal : literal,
		}
	}

	//	Display the literal token in the instruction but make \r
	//	control codes are made displayable
	fn display_literal(&self) -> String{
		if self.literal.token_type == TokenType::STRING{
			let re = Regex::new(r"([\r])").unwrap();
			let result = re.replace_all(&self.literal.token_value, r"\r");
			format!("{}",result)
		}else{
			self.literal.token_value.clone()
		}
	}
}

impl fmt::Display for MachineInstruction{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.opcode{
			Opcode::Break => if self.qualifier[0] > 0{
				write!(f,"{} frame: {} block:{} address:{} depth:{}", self.opcode, self.function_num, self.block_num, self.address, self.qualifier[0])
			}else{
				write!(f,"{} frame: {} block:{} address:{}", self.opcode, self.function_num, self.block_num, self.address)
			},

			Opcode::Continue => if self.qualifier[0] > 0{
				write!(f,"{} frame: {} block:{} address:{} depth:{}", self.opcode, self.function_num, self.block_num, self.address, self.qualifier[0])
			}else{
				write!(f,"{} frame: {} block:{} address:{}", self.opcode, self.function_num, self.block_num, self.address)
			},

			Opcode::FunctionCall =>  write!(f,"{} ({}) arg count={} is_statement={}", self.opcode, self.literal.token_value, self.qualifier[0], self.qualifier[1]),
			Opcode::Diag => write!(f,"{} {}",self.opcode, self.literal.token_value),
			Opcode::FetchIndexed => {
				if self.qualifier.len() > 0{
					write!(f,"{} @{},{},{} indices={} ({})",self.opcode, self.function_num, self.block_num, self.address, self.qualifier[0], self.display_literal())
				}else{
					write!(f,"{} @{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal())
				}
			}
			Opcode::Foreach => {
				write!(f,"{} target: {},{} collection: {},{} index: {},{} exit: {}"
						, self.opcode
						, self.block_num
						, self.address
						, self.qualifier[3]
						, self.qualifier[4]
						, self.qualifier[1]
						, self.qualifier[2]
						, self.qualifier[0]					
					)				
			}

			_=>{
				match self.opcode_mode{
					OpcodeMode::NONE 		=> write!(f,"{}",self.opcode),
					OpcodeMode::Var			=> write!(f,"{} @{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::VarRef		=> write!(f,"{} &{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::Arg			=> write!(f,"{} ^{},{},{}",self.opcode, self.function_num, self.block_num, self.address),
					OpcodeMode::Function	=> write!(f,"{} target {} frame number {} parameter count {}",self.opcode, self.display_literal(), self.function_num, self.qualifier[0]),
					OpcodeMode::Lit 		=> write!(f,"{}(lit) \"{}\"",self.opcode, self.display_literal()),
					OpcodeMode::Builtin		=> write!(f,"{} '{}'",self.opcode, self.display_literal()),
					OpcodeMode::Jump		=> write!(f,"{} *{}",self.opcode, self.address),
					OpcodeMode::Bl			=> write!(f,"{} rtn={}:{} qual={:?}", self.opcode, self.block_num, self.address, self.qualifier),
					OpcodeMode::Update		=> write!(f,"{}(update) {},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::UpdateIndexed => write!(f,"{}(update indexed) {},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::UpdateStructElement => write!(f,"{} #:{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::UpdateIndexedStructElement => write!(f,"{} #:#{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::Alloc		=> write!(f,"{} {},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.display_literal()),
					OpcodeMode::Array		=> write!(f,"{}(array) {},{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.qualifier.len(), self.display_literal()),
					OpcodeMode::Dict		=> write!(f,"{}(dict) {},{},{},{} ({})",self.opcode, self.function_num, self.block_num, self.address, self.qualifier.len(), self.display_literal()),
					OpcodeMode::Internal	=> write!(f,"{}", self.opcode),
				}
			},
		}

	}
}

pub fn token_type_to_opcode(t : TokenType) -> Opcode{
	match t{
		TokenType::LOR 				=> Opcode::Lor,
		TokenType::LAND 			=> Opcode::Land,
	
		TokenType::BITWISE_OR	 	=> Opcode::BwOr,
		TokenType::BITWISE_AND 		=> Opcode::BwAnd,
	
		TokenType::PP 				=> Opcode::Inc,
		TokenType::MM 				=> Opcode::Dec,
	
		TokenType::UMINUS 			=> Opcode::Uminus,			// negation

		TokenType::ADD 				=> Opcode::Add,
		TokenType::SUB 				=> Opcode::Sub,
		TokenType::MUL 				=> Opcode::Mul,
		TokenType::DIV 				=> Opcode::Div,
		TokenType::MOD 				=> Opcode::Mod,

		TokenType::ASG_ADD_EQ 		=> Opcode::AddEq,
		TokenType::ASG_SUB_EQ 		=> Opcode::SubEq,
		TokenType::ASG_MUL_EQ 		=> Opcode::MulEq,
		TokenType::ASG_DIV_EQ 		=> Opcode::DivEq,
		TokenType::ASG_MOD_EQ 		=> Opcode::ModEq,
		TokenType::ASG_APPEND_EQ	=> Opcode::AppendEq,		// concat equal
		TokenType::ASG_OR_EQ		=> Opcode::OrEq,
		TokenType::ASG_AND_EQ		=> Opcode::AndEq,

		TokenType::CONCAT			=> Opcode::Concat,

		//	These are Simple Statements
		TokenType::PRINT 			=> Opcode::Print,
		TokenType::EPRINT			=> Opcode::Eprint,
		TokenType::PRINTLN			=> Opcode::Println,
		TokenType::EPRINTLN			=> Opcode::Eprintln,

		TokenType::BREAK			=> Opcode::Break,
		TokenType::CONTINUE			=> Opcode::Continue,
		TokenType::RETURN			=> Opcode::Return,
		TokenType::EXIT				=> Opcode::Exit,

		TokenType::LT 				=> Opcode::Lt,
		TokenType::GT				=> Opcode::Gt,
		TokenType::LE				=> Opcode::Le,
		TokenType::GE				=> Opcode::Ge,
		TokenType::NE 				=> Opcode::Ne,
		TokenType::EQ 				=> Opcode::Eq,

		TokenType::COMMA			=> Opcode::IncArgCount,
		TokenType::ARG_SEPARATOR	=> Opcode::IncArgCount,
		TokenType::LIST_SEPARATOR	=> Opcode::IncArgCount,

		TokenType::DAMNIT			=> Opcode::Damnit,
		TokenType::LENGTH_OF		=> Opcode::LengthOf,

		TokenType::NEW_COLLECTION	=> Opcode::PushNewCollection,

	_ => abend!(format!("Error from MachinInstruction.token_type_to_opcode: token type {} not implemented", t)),
	}
}
