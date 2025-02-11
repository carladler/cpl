use std::fmt;

/****************************************
****	Opcode
*****************************************/
#[derive(PartialEq, Copy, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum Opcode{
	Nop,
	Push,
	PushNewCollection,
	Pop,
	Add,
	Sub,
	Mul,
	Div,
	Mod,
	FunctionCall,
	FetchIndexed,
	Inc,
	Dec,
	Lor,
	Land,
	Uminus,
	Concat,

	Print,
	Eprint,
	Println,
	Eprintln,

	Exit,
	
	BwAnd,
	BwOr,
	BlockBegin,
	BlockEnd,
	IncArgCount,
	Return,
	J,
	Jt,
	Jf,
	Bl,			// branch and link
	Break,
	Continue,
	Update,
	Append,		// add a value to an array
	Insert,		// add a value to a dictionary
	Alloc,

	// Assignment operators
	AddEq,
	SubEq,
	DivEq,
	MulEq,
	ModEq,
	AppendEq,
	OrEq,
	AndEq,

	// Comparison Operators
	Lt,
	Gt,
	Le,
	Ge,
	Ne,
	Eq,

	//	Unary operators
	Damnit,
	LengthOf,

	//	Kitchen Sink Operators
	Foreach,

	//	Special purpose
	Diag,
}

impl fmt::Display for Opcode{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	match self {
			Opcode::Nop						=> write!(f,"Nop"),
			Opcode::Push					=> write!(f,"Push"),
			Opcode::PushNewCollection		=> write!(f,"PushNewCollection"),
			Opcode::Pop						=> write!(f,"Pop"),
			Opcode::Add						=> write!(f,"Add"),
			Opcode::Sub						=> write!(f,"Sub"),
			Opcode::Mul						=> write!(f,"Mul"),
			Opcode::Div						=> write!(f,"Div"),
			Opcode::Mod						=> write!(f,"Mod"),
			Opcode::FunctionCall			=> write!(f,"FunctionCall"),
			Opcode::FetchIndexed			=> write!(f,"FetchIndexed"),
			Opcode::Inc						=> write!(f,"Inc"),
			Opcode::Dec						=> write!(f,"Dec"),
			Opcode::Lor						=> write!(f,"Lor"),
			Opcode::Land					=> write!(f,"Land"),
			Opcode::Uminus					=> write!(f,"Uminus"),
			Opcode::Concat					=> write!(f,"Concat"),

			Opcode::Print					=> write!(f,"Print"),
			Opcode::Eprint					=> write!(f,"Eprint"),
			Opcode::Println					=> write!(f,"Println"),
			Opcode::Eprintln				=> write!(f,"Eprintln"),

			Opcode::Exit					=> write!(f,"Exit"),

			Opcode::BwAnd					=> write!(f,"Bitwise And"),
			Opcode::BwOr					=> write!(f,"Bitwise Or"),
			Opcode::IncArgCount				=> write!(f,"IncArgCount"),
			Opcode::Return					=> write!(f,"Return"),
			Opcode::BlockBegin				=> write!(f,"BlockBegin"),
			Opcode::BlockEnd				=> write!(f,"BlockEnd"),
			Opcode::J						=> write!(f,"J"),
			Opcode::Jt						=> write!(f,"Jt"),
			Opcode::Jf						=> write!(f,"Jf"),
			Opcode::Bl						=> write!(f,"Bl"),

			Opcode::Break					=> write!(f,"Break"),
			Opcode::Continue				=> write!(f,"Continue"),
		
			Opcode::Update					=> write!(f,"Update"),
			Opcode::Append					=> write!(f,"Append"),
			Opcode::Insert					=> write!(f,"Insert"),
			Opcode::Alloc					=> write!(f,"Alloc"),

			Opcode::AddEq					=> write!(f,"AddEq"),
			Opcode::SubEq					=> write!(f,"SubEq"),
			Opcode::DivEq					=> write!(f,"DivEq"),
			Opcode::MulEq					=> write!(f,"MulEq"),
			Opcode::ModEq					=> write!(f,"ModEq"),
			Opcode::AppendEq				=> write!(f,"AppendEq"),
			Opcode::OrEq					=> write!(f,"OrEq"),
			Opcode::AndEq					=> write!(f,"AndEq"),
		
			Opcode::Lt						=> write!(f,"<"),
			Opcode::Gt						=> write!(f,">"),
			Opcode::Le						=> write!(f,"<="),
			Opcode::Ge						=> write!(f,">="),
			Opcode::Ne						=> write!(f,"!="),
			Opcode::Eq						=> write!(f,"=="),

			Opcode::Damnit					=> write!(f,"!"),
			Opcode::LengthOf				=> write!(f,"LengthOf"),

			Opcode::Diag					=> write!(f,"Diag"),

			Opcode::Foreach					=> write!(f,"Foreach"),
	   }
	}
}

/****************************************
****	OpcodeMode
*****************************************/
#[derive(PartialEq, Copy, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum OpcodeMode{
	Lit,
	Var,
	VarRef,
	Arg,
	Function,
	Builtin,					// Builtin functions
	Jump,						// Absolute Jump target address
	//JumpRel,					// Relative Jump target address (i.e. jump location +/- address)
	Bl,							// Branch and Link (light weight function call)
	Update,						// Signals the update of a value (without regard to its type)
	UpdateIndexed,				// signals updates an element of a collection in situ
	UpdateStructElement,		// Signals update of struct element which is really update array element
	UpdateIndexedStructElement,	// Signals update of an array element owned by a struct member
	
	Array,						// Modifies push_collection, and update
	Dict,						// ditto
	Internal,					// instruction generated internally, not via CPL code
	Alloc,						// this is here for documentation/diag only.  Opcode is always Alloc
	NONE,
}
impl fmt::Display for OpcodeMode{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	match self {
			//	These modes indicate pushing values onto the operand stack
			//	they are "Rvalues"
			OpcodeMode::Lit							=> write!(f,"Lit"),
			OpcodeMode::Var							=> write!(f,"Var"),
			OpcodeMode::VarRef						=> write!(f,"VarRef"),
			OpcodeMode::Arg							=> write!(f,"Arg"),

			OpcodeMode::Function					=> write!(f,"Function"),
			OpcodeMode::Builtin						=> write!(f,"Builtin"),
			OpcodeMode::Jump						=> write!(f,"Jump"),
			// OpcodeMode::JumpRel					=> write!(f,"JumpRel"),
			OpcodeMode::Bl							=> write!(f,"Bl"),
		
			//	These modes indicate to update a variable in situ
			OpcodeMode::Update						=> write!(f,"Update"),
			OpcodeMode::UpdateIndexed				=> write!(f,"UpdateIndexed"),
			OpcodeMode::UpdateStructElement			=> write!(f,"UpdateStructElement"),
			OpcodeMode::UpdateIndexedStructElement	=> write!(f,"UpdateIndexedStructElement"),

			OpcodeMode::Array						=> write!(f,"Array"),
			OpcodeMode::Dict						=> write!(f,"Dict"),
			OpcodeMode::Internal					=> write!(f,"Internal"),
			OpcodeMode::Alloc						=> write!(f,"Alloc"),
			OpcodeMode::NONE						=> write!(f,""),
	   }
	}
}
