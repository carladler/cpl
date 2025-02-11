use std::fmt;
use tokenizer::*;
use cli::*;
use codegen::*;
use codeframe::*;
use structmodel::*;
use literalmodel::*;
use macrolib::*;

use std::cell::RefCell;
use names::*;



//#[derive(PartialEq, Copy, Clone, Eq, Hash)]
pub enum StatementType<'a>{
	AssignmentStatement(AssignmentStatement),
	InstantiateStatement(InstantiateStatement),
	While(While),
	ForeachStatement(ForeachStatement),
	If(If),
	Else(Else),
	SimpleStatement(SimpleStatement),
	LoopStatement(LoopStatement),
	FunctionCallStatement(FunctionCallStatement),
	BlockEnd(BlockEnd),				// block begin instructions generated implicitly
	Function(Function<'a>),			// For context purposes Function is statement
	EvalStatement(EvalStatement),
	WhenStatement(WhenStatement),
	OtherwiseStatement(OtherwiseStatement),
	LiteralStatement(LiteralStatement),
}

impl<'a> fmt::Display for StatementType<'a>{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
			StatementType::AssignmentStatement(_) => write!(f, "AssignmentStatement"),
			StatementType::InstantiateStatement(_) => write!(f, "InstantiateStatement"),
			StatementType::While(_) => write!(f, "While"),
			StatementType::ForeachStatement(_) => write!(f, "Foreach"),
			StatementType::If(b) => write!(f, "If ({})", b.has_else),
			StatementType::Else(_) => write!(f, "Else"),
			StatementType::SimpleStatement(_) => write!(f, "Simple"),
			StatementType::LoopStatement(_) => write!(f, "LoopStatement"),
			StatementType::FunctionCallStatement(_) => write!(f, "FunctionCallStatement"),
			StatementType::BlockEnd(_) => write!(f, "BlockEnd"),
			StatementType::Function(_) => write!(f, "Function"),
			StatementType::EvalStatement(_) => write!(f, "Eval"),
			StatementType::WhenStatement(_) => write!(f, "When"),
			StatementType::OtherwiseStatement(_) => write!(f, "Otherwise"),
			StatementType::LiteralStatement(_) => write!(f, "LiteralStatement"),
	   }
	}
}

/****************************************
****	Expression
*****************************************/

pub enum ExpressionContext{
	Initial,					// No context established yet
	Single,						// expression results in a single value
	List,						// multiple expressions in a list (e.g. for function calls)
}

//	An expression occurs because of a triger (e.g. AssignmentStatement, function call, etc.)
//	The expression context indicates whether a list of expressions is possible
//	e.g. if the term of the expression is a function call.  If the context is
//	single, then there is only a single item in the expression list.  If the context is List
//	then there can be 1 to many items in the expression list
pub struct Expression{
	expression_list : Vec<Token>,		// Postfix tokens
}

impl Expression{
	pub fn new(expression : Vec<Token>) -> Expression{
		Expression {
			expression_list : expression,
		}
	}
}

impl fmt::Display for Expression{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut rslt = String::new();
		for e in &self.expression_list{			
			match e.token_type{
				TokenType::STRING => rslt.push_str(&format!(" \"{}:{}\"", e.token_value, e.token_type)),
				_                 => rslt.push_str(&format!("  {}:{}", e.token_value, e.token_type)),
			}
		}
		write!(f,"{}",rslt)
	}
}



/****************************************
****	Program
*****************************************/

pub struct Program<'a>{
	cli : & 'a CLI<'a>,
	pub functions : Vec<Function<'a>>,
	pub global_literals : Vec<LiteralStatement>,

	//	The struct header table is a list of pointers into the structs table
	//	Each element of this table represents a top-level struct
	pub struct_headers : Vec<usize>,

	//	The structs table holds all of the actual struct bodies.  Each element
	//	of this table is a struct with no distinction made between parent or
	//	child; or rather, the relationships between parents and children are
	//	maintained in the struct's member list.
	pub structs : Vec<Struct>,
}

impl<'a> Program<'a>{
	pub fn new(cli : & 'a CLI,) -> Program<'a>{
		Program{
			cli : cli,
			functions : Vec::new(),			// indexed by function_num			
			global_literals : Vec::new(),
			struct_headers : Vec::new(),
			structs : Vec::new(),
		}
	}

	//	Adds a function to the end of the program's function list, making it the
	//	current function context for future updates
	pub fn add_function (&mut self, name : String,  function_entry_flag : bool, function_parameters : Vec<String>, cl_args : & 'a Vec<String>){
		let f : Function = Function::new(self.cli, name, function_entry_flag, function_parameters, cl_args);
		self.functions.push(f);
	}

	pub fn add_literal(&mut self, literal_statement : LiteralStatement){
		self.global_literals.push(literal_statement);
	}

	pub fn add_child_struct(&mut self, name : &str, current_struct_ix : usize) -> usize{
		//	Adding a child struct requires adding a new struct to the structs table and
		//	a new member to its parent containg the index of the newly added struct.

		let struct_ix = self.structs.len();
		//	member list is at top level
		self.structs.push(Struct::new(name));

		//	create a new member (which is the pointer to its child)
		let member = StructMemberType::Substruct(Substruct::new(name, struct_ix));
		//	add the substruct member to the parent's member list.
		self.structs.get_mut(current_struct_ix).unwrap().members.push(&member);

		return struct_ix;
	}

	//	Adds a top level struct to the struct map.  Returns the header index
	//	and the struct index
	pub fn add_struct(&mut self, name : &str) -> (usize,usize) {
		let struct_index = self.structs.len();
		let struct_header_index = self.struct_headers.len();

		//	create a new struct (initially it has no members so it's just the
		//	name)
		let s : Struct = Struct::new(name);

		//	add the struct to the table and its table index to the header
		self.structs.push(s);
		self.struct_headers.push(struct_index);

		//	tell the parser where in the header this struct landed
		(struct_header_index, struct_index)
	}

	//	Adds a member to the struct to the map at context.  If the level is 0 then the context is a structs_list
	//	index.  If the level > 0 then the context is the index in the child's table.
	pub fn add_struct_member(&mut self, name : &str, initializer : &Vec<Token>, struct_ix : usize){
		self.structs.get_mut(struct_ix).unwrap().members.push(&StructMemberType::Field(Field::new(name.to_string(), initializer)));
	}

	pub fn dump_structs(&self){
		let mut header_index = 0;
		for s in &self.structs{
			s.dump_structs(header_index);
			header_index += 1;
		}
	}

	pub fn print(&self){
		println!("Program Functions List");
		for f in &self.functions{
			println!("    {}", f.name);
		}

		println!("Function Details");
		for f in &self.functions{
			f.print();
		}
	}

	pub fn current_function(&mut self) -> &mut Function<'a>{
		let num_functions = self.functions.len();
		if num_functions > 0{
			return self.functions.last_mut().unwrap();
		}else{
			abend!(format!("NO CURRENT FUNCTION"));
		}
	}
}

/****************************************
****	Function
*****************************************/

pub struct Function<'a>{
	//cli : & 'a CLI<'a>,
	pub name : String,
	pub function_parameters : Vec<String>,

	//	Where all of the statements that implement the function end up.  The
	//	Code Generator derives all of the pseudo machine instructions from this
	//	list
	pub block : StatementBlock<'a>,

	pub function_entry_flag : bool,
	pub cl_args : & 'a Vec<String>,
}

impl<'a>Function<'a>{
	pub fn new(cli : & 'a CLI<'a>, name : String, function_entry_flag:bool, function_parameters : Vec<String>, cl_args : & 'a Vec<String>) -> Function<'a>{
		Function{
			//cli : cli,
			name : name,
			function_parameters : function_parameters,
			block : StatementBlock::new(cli),
			function_entry_flag : function_entry_flag,
			cl_args : cl_args,
		}
	}

	pub fn print(&self){
		println!("Function name: {}",self.name);
		println!("Function Parameters");
		for p in &self.function_parameters{
			println!("   {}",p);
		}
	}

	pub fn generate_code_from_function(&mut self, generator : &mut Generator, function_num : usize){
		generator.gen_function_from_generator(self, function_num);
	}

	pub fn add_function_to_frame(&mut self, generator : &mut Generator){
		generator.add_function_to_frame(self);
	}

	pub fn add_statement(&mut self, s : StatementType<'a>) -> usize{
		return self.block.add_statement(s);
	}

	//	Set the else flag in the model indicating that this IF statement
	//	does have an else clause
	pub fn set_else_flag(&mut self, if_model_context : usize, predicted_block_number : usize){
		self.block.set_else_flag(if_model_context, predicted_block_number);
	}

	pub fn add_when_to_when_list(&mut self, eval_model_context : usize){
		self.block.add_when_to_when_list(eval_model_context);
	}
}



/****************************************
****	StatementBlock
*****************************************/

//	All of the statements added to the model
pub struct StatementBlock<'a>{
	cli : & 'a CLI<'a>,
	pub statements : Vec<StatementType<'a>>,
}


impl<'a> StatementBlock<'a>{
	pub fn new(cli : & 'a CLI<'a>) -> StatementBlock<'a>{
		StatementBlock{
			cli : cli,
			statements : Vec::new(),
		}
	}


	//	Returns where in the model this statement landed.  We normally don't
	//	need this information except in the case of IF because we have to
	//	get back to the IF when the else is encountered
	pub fn add_statement(&mut self, statement : StatementType<'a>) -> usize{
		self.statements.push(statement);
		return self.statements.len() - 1;
	}

	pub fn set_else_flag (&mut self, if_model_context :usize, predicted_block_number : usize){
		if if_model_context > self.statements.len(){
			abend!(format!("from set_else_flag: It appears you've included an else statement without a parent if.  Might be some other issue too."));
		}

		let statement = self.statements.get_mut(if_model_context).unwrap();
		if let StatementType::If(s) = statement {
			if self.cli.is_debug_bit(TRACE_STATEMENT_ADD){
				println!("{}",s);
			}
			s.has_else = true;
			s.else_block_num = predicted_block_number;
		}else{
			abend!(format!("from set_else_flag: Houston we have a problem.  This was supposed to be an IF statement but it was: {} ", self.statements.get_mut(if_model_context).unwrap()));
		}
	}

	pub fn add_when_to_when_list (&mut self, eval_model_context :usize){
		let last_added = self.statements.len() - 1;
		match self.statements.get_mut(eval_model_context).unwrap(){
			StatementType::EvalStatement(s) => s.when_list.push(last_added),
			_ => {
				abend!(format!("from add_when_to_when_list: Houston we have a problem.  This was supposed to be an EVAL statement but it was: {} ", self.statements.get_mut(eval_model_context).unwrap()));
			}
		}
	}
}

/****************************************
****	BlockEnd
*****************************************/
//  the BlockEnd instruction is, essentially, a return from a subroutine
//	It'll pop an operand block and, if the block_stack isn't empty
//	pop it's top and jump to that address (block number, address)
//
//	There isn'a BlockBegin statement.  It is assumed that any "blockable"
//	statement (Kind = LOOP, WHILE, etc) will generate it's own BlockBegin sequence
//	which will create an operand block.
pub struct BlockEnd{
	//cli : & 'a CLI<'a>,
	end_of_function_flag : bool,				// if true then this block end is for end of function
}
impl BlockEnd{
	pub fn new(end_of_function_flag : bool) -> BlockEnd{
		BlockEnd {
			//cli : cli,
			end_of_function_flag : end_of_function_flag,
		}
	}
}

/****************************************
****	While
*****************************************/
pub struct While{
	//cli : & 'a CLI<'a>,
	pub condition : Expression,
}
impl While{
	pub fn new(condition : Vec<Token>) -> While{
		While {
			//cli : cli,
			condition : Expression::new(condition),
		}
	}
}

/****************************************
****	Foreach
*****************************************/
pub struct ForeachStatement{
	//cli : & 'a CLI<'a>,
	pub target : Token,
	pub source_type : TokenType,
	pub source: Vec<Token>,
}
impl ForeachStatement{
	pub fn new(target : Token, source_type : TokenType, source : Vec<Token>) -> ForeachStatement{
		ForeachStatement {
			//cli : cli,
			target : target,
			source_type : source_type,
			source: source,
		}
	}
}

impl fmt::Display for ForeachStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Foreach: {} {}",self.target.token_value,token_list_text(&self.source))
	}
}


/****************************************
****	Eval
*****************************************/


pub struct EvalStatement{
	//cli : & 'a CLI<'a>,
	//	eval <target expression>
	pub target : Expression,

	//	a list of statement numbers for the when statements
	//	associated with this eval
	pub when_list : Vec<usize>,
}

impl EvalStatement{
	pub fn new(target : Vec<Token>) -> EvalStatement{
		EvalStatement {
			// cli : cli,
			target : Expression::new(target),
			when_list : Vec::new(),
		}
	}
}

impl fmt::Display for EvalStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Eval: {}",token_list_text(&self.target.expression_list))
	}
}


/****************************************
****	WHEN
*****************************************/

pub struct WhenStatement{
	//cli : & 'a CLI<'a>,
	pub when_expression : Expression,
	pub eval_model_context : usize,
}

impl WhenStatement{
	pub fn new(when_expression : Vec<Token>, eval_model_context : usize) -> WhenStatement{
		WhenStatement {
			// cli : cli,
			when_expression : Expression::new(when_expression),
			eval_model_context : eval_model_context
		}
	}
}

impl fmt::Display for WhenStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"When: {}",token_list_text(&self.when_expression.expression_list))
	}
}


/****************************************
****	Otherwise
*****************************************/

//	An otherwise statement doesn't do much except mark the place in the
//	model where the otherwise clause of an EVAL statement must be
//	generated

pub struct OtherwiseStatement{
	// cli : & 'a CLI<'a>,
}
impl OtherwiseStatement{
	pub fn new() -> OtherwiseStatement{
		OtherwiseStatement {
			// cli : cli,
		}
	}
}

impl fmt::Display for OtherwiseStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Otherwise")
	}
}


/****************************************
****	If
*****************************************/


pub struct If{
	//cli : & 'a CLI<'a>,
	pub condition : Expression,
	pub has_else : bool,
	pub else_block_num : usize,
}

impl If{
	pub fn new(condition : Vec<Token>) -> If{
		If {
			// cli : cli,
			condition : Expression::new(condition),
			has_else : false,
			else_block_num : 0,
		}
	}
}

impl fmt::Display for If{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "If {} [has_else:{}]",token_list_text(&self.condition.expression_list),self.has_else)
	}
}

/****************************************
****	Else
*****************************************/

//	An else statement doesn't do much except mark the place in the
//	model where the else clause of an If statement must be
//	generated

pub struct Else{
	// cli : & 'a CLI<'a>,
}
impl Else{
	pub fn new() -> Else{
		Else {
			// cli : cli,
		}
	}
}

impl fmt::Display for Else{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Else")
	}
}

/****************************************
****	InstantiateStatement
*****************************************/

pub struct InstantiateStatement{
	//cli : & 'a CLI<'a>,
	pub instantiated_struct : Token,
	pub struct_name : Token,
}
impl InstantiateStatement{
	pub fn new(instantiated_struct : &Token, struct_name : &Token) -> InstantiateStatement{
		InstantiateStatement {
			//cli : cli,
			instantiated_struct : instantiated_struct.clone(),
			struct_name : struct_name.clone(),
		}
	}
}

impl fmt::Display for InstantiateStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"InstantiateStatement: {}=new {}",self.instantiated_struct.token_value, self.struct_name.token_value)
	}
} 

/****************************************
****	AssignmentStatement
*****************************************/

pub struct AssignmentStatement{
	//cli : & 'a CLI<'a>,
	pub target : Token,
	pub target_index_expression : Expression,
	pub op : Token,
	pub expression : Expression,
}
impl AssignmentStatement{
	pub fn new(target : Token, op : Token, target_index_expression : Vec<Token>, expression : Vec<Token>) -> AssignmentStatement{
		AssignmentStatement {
			//cli : cli,
			target : target,					// target of the AssignmentStatement
			target_index_expression : Expression::new(target_index_expression),
			op : op,							// AssignmentStatement operator of some kine
			expression: Expression::new(expression),
		}
	}
}

impl fmt::Display for AssignmentStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"AssignmentStatement: {}{}{}",self.target.token_value,self.op,token_list_text(&self.expression.expression_list))
	}
}

/****************************************
****	SimpleStatement
*****************************************/

pub struct SimpleStatement{
	// cli : & 'a CLI<'a>,
	pub token : Token,
	pub expression : Expression,
}

impl SimpleStatement{
	pub fn new(token:Token, expression : Vec<Token>) -> SimpleStatement{
		SimpleStatement{
			//cli : cli,
			token : token,
			expression : Expression::new(expression),
		}
	}
}

impl fmt::Display for SimpleStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"SimpleStatement: {} {}",self.token.token_value,token_list_text(&self.expression.expression_list))
	}
}


/****************************************
****	LoopStatement
*****************************************/

pub struct LoopStatement{
	// cli : & 'a CLI<'a>,
}

impl LoopStatement{
	pub fn new() -> LoopStatement{
		LoopStatement{
		}
	}
}

impl fmt::Display for LoopStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"Loop")
	}
}

/****************************************
****	Function Call Statement
*****************************************/

pub struct FunctionCallStatement{
	//cli : & 'a CLI<'a>,
	pub function_name : Token,
	pub expression : Expression,
	pub argument_count : usize,
}
impl FunctionCallStatement{
	pub fn new(function_name : Token, expression : Vec<Token>, argument_count : usize) -> FunctionCallStatement{
		FunctionCallStatement {
			//cli : cli,
			function_name : function_name,		// target of the call
			expression: Expression::new(expression),
			argument_count : argument_count,
		}
	}
}
impl fmt::Display for FunctionCallStatement{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"FunctionCallStatement: {} ({}) [{}]",self.function_name,token_list_text(&self.expression.expression_list), self.argument_count)
	}
}


/****************************************
****	Generator (front end)
*****************************************/


pub struct Generator<'a>{
	cli : & 'a CLI<'a>,
	pub code_gen : CodeGen<'a>,

}

impl<'a> Generator<'_>{
	pub fn new(cli : & 'a CLI<'a>, names : & 'a RefCell<Names>) -> Generator<'a>{
		Generator{
			cli : cli,
			code_gen : CodeGen::new(cli,names),
		}
	}

	pub fn add_structs_to_struct_map(&mut self, structs : &Vec<Struct>){
		self.code_gen.add_structs_to_struct_map(structs);
	}

	pub fn add_global_literals(&mut self, global_literals : &Vec<LiteralStatement>){
		self.code_gen.add_global_literals(global_literals);
	}

	pub fn get_frames(&self) -> FrameMap{
		self.code_gen.get_frames()
	}

	pub fn add_function_to_frame(&mut self, function : &mut Function){
		self.code_gen.add_function_to_frame(&function.name, &function.function_parameters, function.function_entry_flag);
	}

	pub fn gen_function_from_generator (&mut self, function : &mut Function, function_num : usize){
		//if self.cli.is_debug_bit(TRACE_CODE_GEN){println!("modelstructs:gen_function {}", function.name);}

		//	does a bunch:  generate codes that mark block begin and intializes frames and symbol table
		self.code_gen.gen_function(&function.name, &function.function_parameters, function.function_entry_flag, function_num, &function.cl_args);

		//	as we generate statements for this function, if we see a "return" then set
		//	the return flag.
		let mut return_flag : bool;
		return_flag = false;

		let mut _statement_num = 0;

		for statement_type in &mut function.block.statements{
			_statement_num += 1;
			
			match statement_type{
				StatementType::SimpleStatement(t) => {
					if t.token.token_type == TokenType::RETURN{
						return_flag = true;
					}
					self.code_gen.gen_simple(&t.token, &t.expression.expression_list, function_num);
				}
				StatementType::LoopStatement(_) => self.code_gen.gen_loop(function_num),
				StatementType::AssignmentStatement(t) => self.code_gen.gen_assignment(&t.target, &t.op, &t.target_index_expression.expression_list, &t.expression.expression_list, function_num),
				StatementType::InstantiateStatement(t) => self.code_gen.gen_struct_instantiate(&t.instantiated_struct, &t.struct_name, function_num),
				StatementType::FunctionCallStatement(t) => self.code_gen.gen_function_call_statement(&t.function_name, &t.expression.expression_list, function_num, t.argument_count),
				StatementType::While(t) => self.code_gen.gen_while(&t.condition.expression_list, function_num),
				StatementType::ForeachStatement(t) => self.code_gen.gen_foreach(&t.target, &t.source_type, &t.source, function_num),
				StatementType::If(t) => {
					self.code_gen.gen_if(&t.condition.expression_list, t.has_else, t.else_block_num, function_num);
				},

				StatementType::Else(_) => self.code_gen.gen_else(function_num),
				StatementType::Function(_) => {},
				StatementType::BlockEnd(b) =>{
					//	If this is the end of function and we haven't seen a RETURN then generate a synthetic return
					//	And skip the block_end because the return does essentially the same thing
					//	NOTE:  we must do it in this order because the BlockEnd removes the operand block and
					//	we still need the operand block to generate the RETURN (pushing synthetic token)
					if b.end_of_function_flag && !return_flag{
						let return_token = Token::new2(TokenType::RETURN, "Return".to_string(), 999, "Synthetic Return".to_string(), TokenCategory::KeywordSimple);						
						let mut expression : Vec<Token> = Vec::new();
						expression.push(Token::new2(TokenType::STRING, "$$Synthetic$$".to_string(), 999, "Synthetic Return".to_string(), TokenCategory::Factor));
						self.code_gen.gen_simple(&return_token, &expression, function_num);
						continue;
					}
					//	And then end the block
					self.code_gen.gen_block_end(function_num);		
				},
				StatementType::EvalStatement(t) => self.code_gen.gen_eval(&t.target.expression_list, t.when_list.len(), function_num),
				StatementType::WhenStatement(t) => self.code_gen.gen_when(&t.when_expression.expression_list, function_num),
				StatementType::OtherwiseStatement(_) => self.code_gen.gen_otherwise(function_num),
				StatementType::LiteralStatement(t) => self.code_gen.gen_literal_statement(&t.literal_id, &t.literal_value),

				//_ => abend!(format!("Unable to generate code for {}", statement_type)),
			}
		}


		if self.cli.is_debug_bit(TRACE_CODE_GEN){
			println!("gen_function ENDING calling gen_block_end");
		}
	}

	pub fn code_dump(&self){
		self.code_gen.code_dump();
	}

	pub fn get_entry_frame_number(&self) -> Option<usize>{
		Some(self.code_gen.get_entry_frame_number())
	}

	pub fn get_frame_count(&self) -> usize{
		return self.code_gen.get_frame_count();
	}
}