// use std::fmt;
use cli::*;
use modelstructs::*;
use tokenizer::*;
use codeframe::*;
use literalmodel::*;

use std::cell::RefCell;
use names::*;


pub struct Model<'a>{
	cli : & 'a CLI<'a>,
	program : Program<'a>,
	generator : Generator<'a>,
	predicted_block_number : usize,
}

impl<'a> Model<'a>{
	pub fn new(cli : & 'a CLI, names : & 'a RefCell<Names>) -> Model<'a>
	{
		 Model{
			cli : cli,
			program : Program::new(cli),
			generator : Generator::new(cli, names),
			predicted_block_number : 0,
		}
	}

	pub fn get_predicted_block_number(&self) -> usize{
		self.predicted_block_number
	}
	
	pub fn get_frames(&self) -> FrameMap{
		self.generator.code_gen.get_frames()
	}

	pub fn parameters_text(&self, parameters : &Vec<String>) -> String{
		let mut rtn = String::new();
		if parameters.len() > 0{
			rtn.push_str(&parameters[0]);
		}else{
			rtn.push_str("NONE");
			return rtn.clone();
		}

		let mut i = 1;
		while i < parameters.len(){
			rtn.push_str(", ");
			rtn.push_str(&parameters[i]);
			i += 1;
		}

		rtn.clone()
	}

	pub fn expression_text_value(&self, token : &Token) -> String{
		if token.token_type == TokenType::STRING{
			format!("\"{}\"",token.token_value)
		}else{
			token.token_value.clone()
		}
	}

	pub fn expression_text(&self, expression : &Vec<Token>) -> String{
		let mut rtn = String::new();
		if expression.len() > 0{
			rtn.push_str(&self.expression_text_value(&expression[0]));
		}else{
			rtn.push_str("NONE");
			return rtn.clone();
		}

		let mut i = 1;
		while i < expression.len(){
			rtn.push_str(" ");
			rtn.push_str(&self.expression_text_value(&expression[i]));
			i += 1;
		}

		rtn.clone()
	}

	pub fn add_block_end(&mut self, end_of_function_flag : bool){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_block_end");}
		let s = BlockEnd::new(end_of_function_flag);
		let f = self.program.current_function();
		f.add_statement(StatementType::BlockEnd(s));
	}

	//	add a new function and make it the current function context (i.e. function_list.last())
	pub fn add_function(&mut self, function_name:String, function_entry_flag : bool, function_parameters:Vec<String>, cl_args : & 'a Vec<String>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_function: {} Entry={} Pameters={}", function_name, function_entry_flag, self.parameters_text(&function_parameters));}
		self.predicted_block_number = 0;
		self.program.add_function(function_name, function_entry_flag, function_parameters, cl_args);
	}

	//	returns header index and struct index
	pub fn add_struct(&mut self, struct_name : &str) -> (usize,usize){
		let rslt = self.program.add_struct(struct_name);
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_struct {} header index={} struct_index={}",struct_name,rslt.0,rslt.1);}
		rslt
	}

	pub fn add_child_struct(&mut self, struct_name : &str, level : usize) -> usize{
		let struct_index = self.program.add_child_struct(struct_name, level);
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_child_struct {} struct index={} level={}",struct_name, struct_index, level);}
		struct_index
	}

	pub fn add_struct_member(&mut self, member_name : &str, initializer : &Vec<Token>, struct_ix : usize){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_struct_member {} init={:?} struct index={}",member_name, token_list_text(initializer), struct_ix);}
		self.program.add_struct_member(member_name, initializer, struct_ix);
	}

	pub fn add_simple_statement(&mut self, token : Token, expression : Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_simple_statement: {} expression: {}", token.token_type, self.expression_text(&expression));}
		let s = SimpleStatement::new(token, expression);
		let f = self.program.current_function();
		f.add_statement(StatementType::SimpleStatement(s));		
	}

	pub fn add_eval_statement(&mut self, target : Vec<Token>) -> usize {
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_eval {}", token_list_text(&target));}
		self.predicted_block_number += 1;
		let s = EvalStatement::new(target);
		let f = self.program.current_function();
		return f.add_statement(StatementType::EvalStatement(s));		
	}

	pub fn add_when_statement(&mut self, when_expression : Vec<Token>, eval_model_context : usize){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_when {}",token_list_text(&when_expression));}
		self.predicted_block_number += 1;
		let s = WhenStatement::new(when_expression, eval_model_context);
		let f = self.program.current_function();
		f.add_statement(StatementType::WhenStatement(s));

		//	Add this when to it's parent eval when_list
		f.add_when_to_when_list(eval_model_context);		
	}

	pub fn add_otherwise_statement(&mut self){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_otherwise");}
		self.predicted_block_number += 1;
		let f = self.program.current_function();
		let s = OtherwiseStatement::new();
		f.add_statement(StatementType::OtherwiseStatement(s));
	}

	pub fn add_loop_statement(&mut self){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_loop_statement");}
		self.predicted_block_number += 1;
		let s = LoopStatement::new();
		let f = self.program.current_function();
		f.add_statement(StatementType::LoopStatement(s));		
	}

	pub fn add_struct_instantiate(&mut self, instantiated_struct : &Token, struct_name : &Token){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {
			println!("Model.add_struct_instantiate: {}=new {}", instantiated_struct.token_value, struct_name);
		}
		let s = InstantiateStatement::new(instantiated_struct, struct_name);
		let f = self.program.current_function();
		f.add_statement(StatementType::InstantiateStatement(s));
	}

	pub fn add_assignment_statement(&mut self, assignment_target:Token, assignment_op:Token, target_index_expression : &Vec<Token>, expression : &Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {
			if target_index_expression.len() == 0{
				println!("Model.add_assignment_statement: {}{}{}", assignment_target.token_value, assignment_op.token_value, token_list_text(&expression));
			}else{
				println!("Model.add_assignment_statement: {}[{}]{}{}", assignment_target.token_value, token_list_text(&target_index_expression), assignment_op.token_value, token_list_text(&expression));
			}
		}
		let s = AssignmentStatement::new(assignment_target, assignment_op, target_index_expression.clone(), expression.clone());
		let f = self.program.current_function();
		f.add_statement(StatementType::AssignmentStatement(s));
	}

	pub fn add_function_call_statement(&mut self, function_name:Token, expression : Vec<Token>, argument_count : usize){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_function_call_statement, {}({}) argument_count={}", function_name.token_value, token_list_text(&expression), argument_count);}
		let s = FunctionCallStatement::new(function_name, expression, argument_count);
		let f = self.program.current_function();
		f.add_statement(StatementType::FunctionCallStatement(s));
	}

	pub fn add_while_statement(&mut self, condition : Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_while_statement");}
		self.predicted_block_number += 1;
		let s = While::new(condition);
		let f = self.program.current_function();
		f.add_statement(StatementType::While(s));
	}

	pub fn add_foreach_statement(&mut self, target : Token, source_type : TokenType,  source : Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_foreach_statement: target:{} source{}", target, source[0]);}
		self.predicted_block_number += 1;
		let s = ForeachStatement::new(target, source_type, source);
		let f = self.program.current_function();
		f.add_statement(StatementType::ForeachStatement(s));
	}

	pub fn add_if_statement(&mut self, condition : Vec<Token>) -> usize{
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_if_statement ({:?})",self.expression_text(&condition));}
		self.predicted_block_number += 1;
		let s = If::new(condition);
		let f = self.program.current_function();
		return f.add_statement(StatementType::If(s));
	}

	//	We've seen if cond { ... }else{ ... } so we need to set the has_else flag
	//	on the IF statement via the if_model_context_value
	pub fn add_else_statement(&mut self, if_model_context : usize){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("    Model.add_else_statement. context={}", if_model_context);}

		//  When codegen sees this it'll add a new block.  It's value will be
		//	the predicted_block_number
		self.predicted_block_number += 1;
		let s = Else::new();
		let f = self.program.current_function();

		//	Add the ELSE statement to the model
		f.add_statement(StatementType::Else(s));

		//	And tell the model to update the else flag and
		//	block_number
		f.set_else_flag(if_model_context, self.predicted_block_number);
	}

	pub fn add_global_literal(&mut self, literal_id : Token, literal_value : Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_global_literal: {} literal_value: {}", literal_id, token_list_text(&literal_value));}
		let s = LiteralStatement::new(literal_id, literal_value);
		self.program.add_literal(s);
	}

	pub fn add_literal_statement(&mut self, literal_id : Token, literal_value : Vec<Token>){
		if self.cli.is_debug_bit(TRACE_MODEL_ADD) {println!("Model.add_literal_statement: {} literal_value: {}", literal_id, token_list_text(&literal_value));}
		let s = LiteralStatement::new(literal_id, literal_value);
		let f = self.program.current_function();
		f.add_statement(StatementType::LiteralStatement(s));		
	}

	pub fn generate_code_from_model (&mut self){
		//  First, add the structs to the symbol table
		self.generator.add_structs_to_struct_map(&self.program.structs);

		//	Then add global literals, if there are any
		self.generator.add_global_literals(&self.program.global_literals);

		//	Next, we need to know what where to start because
		//	elsewhere we've added a bunch of builtin functions to
		//	the frame map.  This call follows a chain of calls that
		//	make your head hurt.  Suffice it say, the struct that "knows"
		//	how many frames there are in the Frame Map is in the genstructs::CodeGen
		//	struct.
		let start_frame = self.generator.get_frame_count();

		//	Add all of the functions to the Frame Map before generating code for
		//	any statements within the function.  I'm hoping this will obviate the 
		//	need for forward functions.  And it seems to work.
		for function in &mut self.program.functions{
			function.add_function_to_frame(&mut self.generator);
		}

		//	Once we've added all of the function names to the Frame Map
		//	we now need to add the code for each of those functions.  Note
		//	that we start at the frame immediately following the last builtin
		//	function we added.
		//
		//	Note that we pass "args" to every function but only the
		//	entry function will use it
		let mut function_num = start_frame;
		for function in &mut self.program.functions{
			function.generate_code_from_function(&mut self.generator, function_num);
			function_num += 1;
		}
	}

	pub fn code_dump(&self){
		self.generator.code_dump();
	}

	pub fn dump_structs(&self){
		self.program.dump_structs();
	}
}

