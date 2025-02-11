use std::collections::HashMap;
use cli::*;
use tokenizer::*;
use macrolib::*;

const COMP_OP_PRECEDENCE : i32 = 4;
const REL_OP_PRECEDENCE : i32 = 1;
const ADD_OP_PRECEDENCE : i32 = 5;
const MUL_OP_PRECEDENCE : i32 = 6;
const UNARY_OP_PRECEDENCE : i32 = 7;
const FUNCTION_CALL_PRECEDENCE : i32 = 8;
const INDEXED_ID_PRECEDENCE : i32 = 8;
const FACTOR_PRECEDENCE : i32 = 5;
const LPAREN_PRECEDENCE : i32 = 2;
const RPAREN_PRECEDENCE : i32 = 2;
const LPAREN_ARG_PRECEDENCE : i32 = 1;
const RPAREN_ARG_PRECEDENCE : i32 = 1;
const RBRACKET_PRECEDENCE : i32 = 4;
const LBRACKET_PRECEDENCE : i32 = 3;
const RBRACE_PRECEDENCE : i32 = 4;
const LBRACE_PRECEDENCE : i32 = 3;
const RINDEX_PRECEDENCE : i32 = 8;
const LINDEX_PRECEDENCE : i32 = 8;

const _LDICT_PRECEDENCE : i32 = 3;
const _RDICT_PRECEDENCE : i32 = 4;
const _LDICT_KV_PRECEDENCE : i32 = 3;
const RDICT_KV_PRECEDENCE : i32 = 4;

const COMMA_PRECEDENCE : i32 = 1;
const LIST_SEPARATOR_PRECEDENCE : i32 = 1;
const ARG_SEPARATOR_PRECEDENCE : i32 = 1;
const LOGICAL_PRECEDENCE : i32 = 5;

const OP_UP_PRECEDENCE : i32 = 1;

const _UNKNOWN_PRECEDENCE : i32 = 0;
const _NO_PRECEDENCE : i32 = 0;

pub struct InfixToPostfix<'a>{
	cli : & 'a CLI<'a>,
	//tokenizer : &'a Tokenizer<'a>,

	//	where the converted infix to postfix expression tokens end up
	postfix_expression : Vec<Token>,

	//	the operator stack used for infix to postfix conversion
	//operator_stack : Vec<Token>,

	token_precedence : HashMap<TokenType, i32>,
}

impl <'a> InfixToPostfix<'a>{
	pub fn new(cli : & 'a CLI<'a>) -> InfixToPostfix <'a>{
		InfixToPostfix{
			cli : cli,
			//tokenizer : Tokenizer::new(cli),

			postfix_expression : Vec::new(),
			//operator_stack : Vec::new(),

			token_precedence : HashMap::from(
				[
					(TokenType::TRUE,LOGICAL_PRECEDENCE),
					(TokenType::FALSE,LOGICAL_PRECEDENCE),

					(TokenType::EQ,COMP_OP_PRECEDENCE),
					(TokenType::GE,COMP_OP_PRECEDENCE),
					(TokenType::LE,COMP_OP_PRECEDENCE),
					(TokenType::NE,COMP_OP_PRECEDENCE),
					(TokenType::GT,COMP_OP_PRECEDENCE),
					(TokenType::LT,COMP_OP_PRECEDENCE),
				
					(TokenType::LOR,REL_OP_PRECEDENCE),
					(TokenType::LAND,REL_OP_PRECEDENCE),
				
					(TokenType::BITWISE_OR,ADD_OP_PRECEDENCE),
					(TokenType::BITWISE_AND,ADD_OP_PRECEDENCE),
				
					(TokenType::PP,UNARY_OP_PRECEDENCE),
					(TokenType::MM,UNARY_OP_PRECEDENCE),
					(TokenType::UMINUS,UNARY_OP_PRECEDENCE),
					(TokenType::UPLUS,UNARY_OP_PRECEDENCE),				
					(TokenType::DAMNIT,UNARY_OP_PRECEDENCE),
					(TokenType::LENGTH_OF,UNARY_OP_PRECEDENCE),

				
					(TokenType::ADD,ADD_OP_PRECEDENCE),
					(TokenType::SUB,ADD_OP_PRECEDENCE),
				
					(TokenType::MUL,MUL_OP_PRECEDENCE),
					(TokenType::DIV,MUL_OP_PRECEDENCE),
					(TokenType::MOD,MUL_OP_PRECEDENCE),
					(TokenType::CONCAT,ADD_OP_PRECEDENCE),
			
					(TokenType::LPAREN,LPAREN_PRECEDENCE),
					(TokenType::RPAREN,RPAREN_PRECEDENCE),

					(TokenType::LPAREN_ARG,LPAREN_ARG_PRECEDENCE),
					(TokenType::RPAREN_ARG,RPAREN_ARG_PRECEDENCE),

					(TokenType::LBRACE,LBRACE_PRECEDENCE),
					(TokenType::RBRACE,RBRACE_PRECEDENCE),

					(TokenType::LBRACKET,LBRACKET_PRECEDENCE),
					(TokenType::RBRACKET,RBRACKET_PRECEDENCE),

					(TokenType::LINDEX,LINDEX_PRECEDENCE),
					(TokenType::RINDEX,RINDEX_PRECEDENCE),

					(TokenType::RDICT_KV,RDICT_KV_PRECEDENCE),

					(TokenType::COMMA,COMMA_PRECEDENCE),
					(TokenType::LIST_SEPARATOR,LIST_SEPARATOR_PRECEDENCE),
					(TokenType::ARG_SEPARATOR,ARG_SEPARATOR_PRECEDENCE),
						
					(TokenType::STRING,FACTOR_PRECEDENCE),			// quoted with either ' or "
					(TokenType::INTEGER,FACTOR_PRECEDENCE),
					(TokenType::FLOAT,FACTOR_PRECEDENCE),
					(TokenType::BOOL,FACTOR_PRECEDENCE),
				
					(TokenType::ID,FACTOR_PRECEDENCE),
					(TokenType::INDEXED_ID,INDEXED_ID_PRECEDENCE),
					
					(TokenType::FUNCTION_CALL(0),FUNCTION_CALL_PRECEDENCE),
				]
			),
		}
	}

	// ************************************************************************
	// ****	Infix to Postfix Conversion
	// ************************************************************************/
	pub fn infix_to_postfix(&mut self, infix : &Vec<Token>) -> &mut Vec<Token>{
		let mut postfix : Vec<Token> = Vec::new();
		let mut operators : Vec<Token> = Vec::new();
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX_DRIVER){
			eprintln!("infix_to_postfix {}",self.infix_expression_text(&infix));
		}

		for t in infix{
			self.i_to_p(t, &mut postfix, &mut operators);
			if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX_DRIVER){
				eprintln!("i_to_p:  --LAST Token: {} (\"{}\")\tpostfix={}\toperators={}",t.token_type, t.token_value, self.token_list_text(&mut postfix), self.token_list_text(&mut operators));
			}
		}

		self.clear_operator_stack(&mut postfix, &mut operators);

		self.postfix_expression.clear();
		self.postfix_expression.append(&mut postfix);

		if self.cli.is_debug_bit(DUMP_POSTFIX_EXPRESSION){
			eprintln!("Infix = {} ", self.infix_expression_text(&infix));
			eprintln!("Postfix: {}", self.postfix_expression_text());
		}

		&mut self.postfix_expression
	}

	fn i_to_p (&mut self, token : &Token, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){

		//  Operands don't move.  That is, they added to the postfix list
		//  as soon as they are seen.
		match token.token_category{
			TokenCategory::RBrace			|
			TokenCategory::RBracket			|
			TokenCategory::RDict_Kv			|
			TokenCategory::BinaryOp 		|
			TokenCategory::UnaryOp  		|
			TokenCategory::RelOp			|
			TokenCategory::IncDec			=> {},

			//	Special cases
			TokenCategory::LParen  | TokenCategory::LParen_Arg  =>{
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p:  Stacking unconditionally: {}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
				}
			    operators.push(token.clone());
			    return;	
		  	},

			TokenCategory::RIndex =>  {
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p:  RIndex move: {}\tpostfix={}\toperators={}",token, self.token_list_text(postfix), self.token_list_text(operators));
				}

				self.move_special_rindex(&token, postfix, operators);
				return;
			},

			TokenCategory::IndexedId => {
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p:  IndexedId move: {}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
				}
				self.op_up(operators);
				self.move_operator(&token, postfix);
				return;
			},

			TokenCategory::RParen_Arg       => {
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p:  RParen_Arg move: {}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
				}
				self.special_case_c(postfix, operators);
				return;
			},

			TokenCategory::RParen			=> {
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p:  RParen move: {}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
				}
				self.clear_operator_stack(postfix, operators);
				return;
			},

			TokenCategory::ListSeparator  	=> {
				self.clear_operator_stack(postfix, operators);
				postfix.push(token.clone());
				return;
			},

			TokenCategory::ArgSeparator		=> {
				//	fn clear_operator_stack_with_stop(&mut self, stop : TokenCategory, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){
				//eprintln!(".... token={} operators: @arg_separator operators = {}\t{}", token.token_value, self.token_list_text(operators),token.line_text);
				self.special_case_a(&token, postfix, operators);
				// postfix.push(token.clone());
				return;
			},
			
			TokenCategory::FunctionCall     => {
				self.special_case_b(&token, operators);
				return;
			},

			//	Ignore this one
			TokenCategory::LIndex => return,

			
			_ => {
					postfix.push(token.clone());
					//eprintln!(".... adding token={}", token);
					return;
				 },
		}

	// *******************************************************************************
	// *********  from here on we know that the input token is an operator  **********
	// *******************************************************************************

		//	if the operator stack is empty we can just push what we found onto it
		if 	operators.is_empty() {
			if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
				eprintln!("i_to_p:  Operators empty stacking: {}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
			}
			operators.push(token.clone());
			return;
		}

		//	if the operator stack has a <op>^ at the top, just add the input
		//	unconditionaly to the operator stack (in effect, <op>^ has infinately small precedence)
		if self.is_op_up(operators.last().unwrap()){
			if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
				eprintln!("i_to_p:  <op>^ at TOS, stacking:{}\tpostfix={}\toperators={}",token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
			}
			operators.push(token.clone());
			return;
		}

		//  If the precedence of the input token is LESS THAN OR EQUAL to the precedence at the
		//	top of the operator stack then move the token at the operator stack to the postfix
		//	expression.  This will ensure that expressions like:
		//
		//		a / b / c / d
		//
		//	end up as:
		//
		//		ab / c / d /
		//
		//	as opposed to:
		//
		//		a b c d / / /
		//
		//  Unless the token on the stack is RINDEX in which case the input is moved
		//
		if self.token_prec(token) <= self.token_prec(operators.last().unwrap()){
			//let last = operators.len() - 1;
			if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
				eprintln!("i_to_p: token='{}'({}) <= operator stack tos: {}", token.token_value, self.token_prec(&token), self.token_list_text(operators));
			}
				
			while !operators.is_empty() && self.token_prec(&token) <= self.token_prec(operators.last().unwrap()){
			
				//	The input token < TOS so we want to move the tos to the postfix expression
				//	but, if TOS is a parenthesis we just kill it and try again
				let operator_token = operators.pop().unwrap();
				self.move_operator(&operator_token, postfix);				
			}

			//	now, either the operator stack is empty or the input token >= operator tos
			//	save the input token and we're done
			operators.push(token.clone());
			if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
				eprintln!("i_to_p: {} stacked, stack={}", token.token_value,self.token_list_text(operators));
			}
			return;	
		}else{			
			// The input token has a greater precedence than the token at the top of the stack,
			// so just push the current token onto the operator stack unless it is a RPAREN
			// in which case just ignore it

			//	now, either the operator stack is empty or the input token >= operator tos
			//	save the input token and we're done
			if token.token_type != TokenType::RPAREN && token.token_type != TokenType::LPAREN {
				operators.push(token.clone());
				if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
					eprintln!("i_to_p: {} stacked, stack={}", token.token_value, self.token_list_text(operators));
				}
			}
		}
	}


	
	// ***********************************************************************
	// ****	Expression Parsing Support Functions
	// ***********************************************************************
	

	//	When INDEXED_ID or FUNCTION_CALL is seen, and the operator
	//	stack is not empty, change token at the top of the stack
	//	from <op> to <op>^ by changing appending a '^' to
	//	the token_value field of the token.
	fn op_up(&mut self, operators : &mut Vec<Token>){
		if operators.is_empty(){ return;}

		let last : usize = operators.len() -1 ;

		operators[last].token_value.push('^');

		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
			eprintln!("i_to_p.op_up Changing <op> to <op>^\toperators={}",self.token_list_text(operators));
		}
	}

	//	remove the "^" from the token_value field at the top of the operators stack
	//	if the stack is empty or the operator is not <op>^ then do nothing
	fn op_down(&mut self, operators : &mut Vec<Token>){
		if operators.is_empty(){
			return;
		}

		//	get index to TOS
		let last : usize = operators.len() - 1;

		//	get the token at TOS
		let mut up = operators[last].clone(); 

		//  get the index to the last character of the value
		let up_loc : usize = up.token_value.len() - 1;

		//	get the last character
		let last_char = up.token_value.chars().nth(up_loc).unwrap();

		//	if the last character isn't a ^ then do nothing
		if last_char != '^'{
			return;
		}

		//	remove the last character of the value
		up.token_value.pop();

		//	replace the token at the TOS
		operators.pop();
		operators.push(up);
	}

	//	If the parmameter token has "^" appended to the value return true
	//	else return false
	fn is_op_up(&self, token : &Token) -> bool{
		let last = token.token_value.len() - 1;
		if token.token_value.chars().nth(last).unwrap() == '^'{
			return true;
		}
		return false;
	}

	fn token_prec (&self, token : &Token) -> i32{
		// eprintln!(".... token_prec {}", token.token_type);
		if let TokenType::FUNCTION_CALL(_) = token.token_type{
			*self.token_precedence.get(&TokenType::FUNCTION_CALL(0)).unwrap()
		}else{
			if self.is_op_up(token){
				return OP_UP_PRECEDENCE;
			}else{
				*self.token_precedence.get(&token.token_type).unwrap()
			}
		}
	}

	//	this is a clearing helper
	fn move_operator(& mut self, token : &Token, postfix : &mut Vec<Token>){
		//eprintln!("............i_to_p: move_operator {}", token);

		//  Don't move these tokens to postfix
		match token.token_type{
			TokenType::LPAREN |
			TokenType::RPAREN |
			TokenType::LPAREN_ARG |
			TokenType::RPAREN_ARG |
			TokenType::ARG_SEPARATOR |
			TokenType::LINDEX
			=> {
				return;
			   },
			_=>{}
		}


		self.trace_infix_to_postfix_moving(&token, "  Moving", TRACE_INFIX_TO_POSTFIX, postfix, line!());
		postfix.push(token.clone());
	}

	//	Just like move_operator except compares token with tos and if token
	//	has a lower percedence then tos move tos instead and replace tos
	//	with input
	fn move_operator_prec(&mut self, token_to_move : &Token, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){
		//  Don't move these tokens to postfix
		match token_to_move.token_type{
			TokenType::LPAREN |
			TokenType::RPAREN |
			TokenType::LPAREN_ARG |
			TokenType::RPAREN_ARG |
			TokenType::ARG_SEPARATOR |
			TokenType::LINDEX
			=> {
				return;
			   },
			_=>{}
		}

		//	next compare the token to move with the token at the top of the
		//	operator stack.  if the token_to_move has a higher precedence
		//	than the tos move it else move the tos
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
			eprintln!("i_to_p:  move_operator_prec: {}\tpostfix={}\toperators={}", token_to_move.token_value, self.token_list_text(postfix), self.token_list_text(operators));
		}

		if operators.is_empty(){
			self.trace_infix_to_postfix_moving(&token_to_move, "  Moving unconditionally", TRACE_INFIX_TO_POSTFIX, postfix, line!());
			postfix.push(token_to_move.clone());
			return;
		}

		if self.token_prec(token_to_move) > self.token_prec(operators.last().unwrap()){
			self.trace_infix_to_postfix_moving(&token_to_move, "  Moving", TRACE_INFIX_TO_POSTFIX, postfix, line!());
			postfix.push(token_to_move.clone());
			return;
		}
		
		//	otherwise, move the one with higher priorty and
		//	leave the one with the lower priority on the
		//	operator stack
		let moveable_token = operators.pop().unwrap();
		self.trace_infix_to_postfix_moving(&moveable_token, "  Moving", TRACE_INFIX_TO_POSTFIX, postfix, line!());
		postfix.push(moveable_token.clone());
		operators.push(token_to_move.clone());
	}

	//	Special Function Call Case A:  input is ARG_SEPARATOR (',').  Move everything from operators TOS down
	//	to, but not including LPAREN_ARG.  If there is nothing to move just return.
	fn special_case_a(&mut self, trigger : &Token, postfix : &mut Vec<Token>, operators : &mut Vec<Token> ){
		if operators.is_empty(){
			// abend!(format!("from special_case_a:  Houston, we have a problem.  Dangling ARG_SEPARATOR"));
			return;
		}

		//	Move until operators are empty (and if it goes empty then we have another
		//	error because the separator should be part of a function call which should
		//	still be on the stack)
		while !operators.is_empty(){
			if operators.last().unwrap().token_type == TokenType::LPAREN_ARG {
				return;
			}

			self.move_operator_prec(&operators.pop().unwrap(), postfix, operators);
		}

		abend!(format!("from special_case_a:  Houston, we have a problem.  LPAREN_ARG not found where it should be.  Triggered by {}",trigger.token_type));
	}

	//	Input is FunctionCall; convert operator at TOS (if there is one) to <op>^ and
	//	push the FC onto the operator stack
	//	The precedence of <op>^ tokens depends on the context.
	fn special_case_b(&mut self, token : &Token, operators : &mut Vec<Token>){
		self.op_up(operators);
		operators.push(token.clone());
	}

	//	input is RPAREN_ARG; move all operators in precedence order down to but not including
	//  <op>^, convert <op>^ to <op>
	fn special_case_c(&mut self, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
			eprintln!("i_to_p:  special_case_c:\tpostfix={}\toperators={}", self.token_list_text(postfix), self.token_list_text(operators));
		}
		while !operators.is_empty(){
			//	If the operator at the top of stack is <op>^ then "down" it
			if self.is_op_up(operators.last().unwrap()){
				self.op_down(operators);
				return;
			}

			let tos = operators.pop().unwrap();
			self.move_operator_prec(&tos, postfix, operators);
		}
	}

	//  Move everything in the operator stack to the postfix stack.  However, we
	//	move things in precednece order (ish)
	fn clear_operator_stack(&mut self, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){
		self.clear_operator_stack_with_stop(TokenCategory::Unknown, postfix, operators)
	}

	//	Move everythnig in the operator stack until we see a token
	//	whose category == the stop category.  We use this to prevent
	//	moving the function call (and maybe some others)
	fn clear_operator_stack_with_stop(&mut self, stop : TokenCategory, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){
		//	first, if there's nothing to move, then get out
		if operators.is_empty(){return}

		self.trace_infix_to_postfix_msg(&format!("Clearing the operators {}", self.token_list_text(operators)), line!());
		self.trace_operator_stack_content(operators);
		self.trace_postfix_expression_content(postfix);

		while !operators.is_empty(){
			//	Stop moving if the stop is known and the current token category is stop
			if operators.last().unwrap().token_category == stop{
				return;
			}

			//	If there's only one token left just move it and be done
			//	unless its a PAREN in which case, just ignore it
			if operators.len() == 1 {
				if operators.last().unwrap().token_category == stop{
					return;
				}else{
					let token = operators.pop().unwrap();
					self.move_operator(&token, postfix);
					return;	
				}
			}

			//	Otherwise, if there is more than one operator on the stack
			//	move it in precdence order (i.e. either move it or the tos
			//	if tos is higher)
			let token = operators.pop().unwrap();
			self.move_operator_prec(&token, postfix, operators);
		}
	}

	//	When input is RINDEX:
	//
	//		move operators to postfix, in order of precedecne, until we see an <op>^
	//		move the parameter (which is supposed to be RINDEX)
	//		if operator stack still not empty then change tos from <op>^ to <op>
	//		After moving everything, and changing the <op>, move the token that
	//		was passed.

	fn move_special_rindex(&mut self, special : &Token, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){

		//	move operators in precedence order until we see a <op>^
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
			eprintln!("i_to_p: Moving operators until <op>^ input:{}\tpostfix={}\tstack={}", special.token_value, self.token_list_text(postfix), self.token_list_text(operators));
		}
		while !operators.is_empty(){
			//	if we're looking at a <op>^, down it and add the special
			if self.is_op_up(operators.last().unwrap()){
				self.op_down(operators);
				break;
			}
			
			//	next compare top two tokens and move the one with higher precedent
			//	unless the 2nd one is a <op>^ in which case just move the top one and
			//	down the second one
			let token = operators.pop().unwrap();
			
			//	but, if there aren't any more operators, just move it and exit loop
			if operators.is_empty(){
				self.move_operator(&token,postfix);
				break;
			}

			//  OR if tos -1 was <op>^ then move the tos and let the <op>^ be 
			//	tos on the next round
			//	exit the loop	
			if self.is_op_up(operators.last().unwrap()){
				self.move_operator(&token,postfix);
				continue;
			}

			//	if the token > token at tos move it and loop
			if self.token_prec(&token) > self.token_prec(operators.last().unwrap()){
				self.move_operator(&token, postfix);
				continue;
			}

			//	otherwise move the tos token then the tos-1 token
			//	and replace the operator at tos with the lower
			//	precedent token.

			//	there is a bug here: what we really need to do is
			//	sort all of the tokens we want to move in precedence order
			//	(high to low) and then just move them.  The bug is
			//	we aren't taking into account tokens farther down the
			//	stack which might have higher precedence then
			//	the one we move.

			let moveable_token = operators.pop().unwrap();
			self.move_operator(&moveable_token, postfix);
			operators.push(token);
		}

		//	when stack is empty, move the special token
		self.trace_infix_to_postfix_msg(&format!("Special clearing, operator stack is empty: '{}'", special.token_value), line!());
		self.move_operator(special, postfix);
	}

	//	When RParen is seen, move higher precedence tokens before lower precedence tokens
	//  until either empty or FUNCTION_CALL.  The twist is "(" is always less than anything so anything
	//	after "(" will get moved.  "(" will always be skipped.

	// fn move_special_rparen(&mut self, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){

	// 	//	move operators in precedence order until we see a Function Call
	// 	if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
	// 		eprintln!("i_to_p: Moving operators until FC or empty\tpostfix={}\tstack={}", self.token_list_text(postfix), self.token_list_text(operators));
	// 	}


	// 	while !operators.is_empty(){
	// 		//	next compare top two tokens and move the one with higher precedent
	// 		//	unless the 2nd one is a <op>^ in which case just move the top one and
	// 		//	down the second one
	// 		let token = operators.pop().unwrap();

	// 		//eprintln!("i_to_p: Moving operators until FC or empty ..{}..\tpostfix={}\tstack={}", token.token_value, self.token_list_text(postfix), self.token_list_text(operators));
			
	// 		//	When we see the FC we're don
	// 		match token.token_type{
	// 			TokenType::FUNCTION_CALL(_) =>{
	// 				return;
	// 			}
	// 			_ => {}
	// 		}

	// 		//	if this was the last operator just move it and return
	// 		if operators.is_empty(){
	// 			self.move_operator(&token,postfix);
	// 			return;
	// 		}

	// 		//	if the token > token at tos move it and loop
	// 		if self.token_prec(&token) > self.token_prec(operators.last().unwrap()){
	// 			self.move_operator(&token, postfix);
	// 			continue;
	// 		}

	// 		//	otherwise move the tos token then the tos-1 token
	// 		//	and replace the operator at tos with the lower
	// 		//	precedent token.

	// 		//	there is a bug here: what we really need to do is
	// 		//	sort all of the tokens we want to move in precedence order
	// 		//	(high to low) and then just move them.  The bug is
	// 		//	we aren't taking into account tokens farther down the
	// 		//	stack which might have higher precedence then
	// 		//	the one we move.

	// 		let moveable_token = operators.pop().unwrap();
	// 		self.move_operator(&moveable_token, postfix);
	// 		operators.push(token);
	// 	}
	// }

	// //	When input is Rparen_arg, move everything, up to and including FC but not more
	// //	and also stop at LPAREN
	// fn move_special_rparen_arg(&mut self, postfix : &mut Vec<Token>, operators : &mut Vec<Token>){

	// 	//	move operators in precedence order until we see a Function Call
	// 	if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
	// 		eprintln!("i_to_p: Moving FC and other operators until empty\tpostfix={}\tstack={}", self.token_list_text(postfix), self.token_list_text(operators));
	// 	}

	// 	while !operators.is_empty(){
	// 		//	if we're looking at an LParen then we're done
	// 		if operators.last().unwrap().token_type == TokenType::LPAREN{
	// 			//eprintln!(".......lparen top");
	// 			return;
	// 		}

	// 		//	otherwise, get the tos
	// 		let token = operators.pop().unwrap();
	// 		//eprintln!(".......current tos {}", token.token_value);

			
	// 		//	and if this was the last operator just move it return
	// 		if operators.is_empty(){
	// 			//eprintln!(".......no more after {}",token);
	// 			self.move_operator(&token,postfix);
	// 			return;
	// 		}

	// 		//	if the token > token at tos move it and loop
	// 		if self.token_prec(&token) > self.token_prec(operators.last().unwrap()){
	// 			//eprintln!(".......{} > {}",token.token_value, operators.last().unwrap().token_value);
	// 			self.move_operator(&token, postfix);
	// 			continue;
	// 		}

	// 		//	otherwise move the tos token then the tos-1 token
	// 		//	and replace the operator at tos with the lower
	// 		//	precedent token.  Unless the the tos was a LPAREN
	// 		//	in which case, move the token and the be done

	// 		//	there is a bug here: what we really need to do is
	// 		//	sort all of the tokens we want to move in precedence order
	// 		//	(high to low) and then just move them.  The bug is
	// 		//	we aren't taking into account tokens farther down the
	// 		//	stack which might have higher precedence then
	// 		//	the one we move.

	// 		if operators.last().unwrap().token_type == TokenType::LPAREN{
	// 			//eprintln!(".......lparen bottom");
	// 			self.move_operator(&token, postfix);
	// 			return;
	// 		}

	// 		//	get the token at tos and move it
	// 		let moveable_token = operators.pop().unwrap();

	// 		//eprintln!(".......moving {} saving {}", moveable_token.token_type, token.token_type);

	// 		self.move_operator(&moveable_token, postfix);

	// 		//	save the token that was tos until the next time through the loop
	// 		operators.push(token);

	// 		//	if we must moved a function call, then done
	// 		if moveable_token.token_category == TokenCategory::FunctionCall{
	// 			//eprintln!(".......moveable is function call so done");
	// 			return;
	// 		}
	// 	}
	// }


	// fn trace_infix_to_postfix(&mut self, token : &Token, text : &str, bit : u32, line : u32){
	// 	if self.cli.is_debug_bit(bit){
	// 		eprintln!("i_to_p: {} '{}' type={} prec={} line={}",text, token.token_value, token.token_type, self.token_prec(&token), line);
	// 	}
	// }

	fn trace_infix_to_postfix_moving(&mut self,  token : &Token, text : &str, bit : u32, postfix : &mut Vec<Token>, line : u32){
		if self.cli.is_debug_bit(bit){
			eprintln!("i_to_p: {} '{}' type={} prec={} line={} postfix={}",text, token.token_value, token.token_type, self.token_prec(&token), line, self.token_list_text(postfix));
		}
	}

	fn trace_infix_to_postfix_msg(&mut self, text : &str, line : u32){
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX){
			eprintln!("i_to_p: {} line={}",text, line);
		}
	}

	fn trace_postfix_expression_content (&self, postfix : &mut Vec<Token>){
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX) {
			eprintln!("i_to_p    postfix = {}", self.token_list_text(postfix));
		}
	}

	fn trace_operator_stack_content (&self, operators : &mut Vec<Token>){
		if self.cli.is_debug_bit(TRACE_INFIX_TO_POSTFIX) {
			eprintln!("i_to_p    operators = {}", self.token_list_text(operators));
		}
	}

	fn infix_expression_text(&self, infix : &Vec<Token>) -> String{
		let mut rtn = String::new();
		for t in infix{
			//	if the token is a string println it in quotes
			if t.token_type == TokenType::STRING{
				rtn.push_str(&format!("\"{}\"",t.token_value));
			}else{
				// if the token is a function call, println it with the argument count
				// otherwise just println it
				match t.token_type{
					TokenType::FUNCTION_CALL(_) => rtn.push_str(&format!("{}", t.token_value)),
					TokenType::LPAREN_ARG | TokenType::RPAREN_ARG => rtn.push_str(&format!(" {} ", t.token_value)),
					TokenType::IDADDR => rtn.push_str(&format!("&{} ", t.token_value)),
					_ => rtn.push_str(&format!("{}",t.token_value)),
				}
			}
		}
		rtn
	}

	//	println the content of a list of tokens
	fn token_list_text (&self, list : &mut Vec<Token>) -> String{
		let mut rtn = String::new();
		for t in list{
			//	if the token is a string println it in quotes
			if t.token_type == TokenType::STRING{
				rtn.push_str(&format!("\"{}\" ",t.token_value));
			}else{
				// if the token is a function call, println it with the argument count
				// if the token is a reference, println it with "&"
				// otherwise just println it
				match t.token_type{
					TokenType::FUNCTION_CALL(_) => rtn.push_str(&format!("{} ", t.token_value)),
					TokenType::IDADDR => rtn.push_str(&format!("&{} ", t.token_value)),
					TokenType::NEW_COLLECTION => rtn.push_str("NEW_COLLECTION "),
					_ => rtn.push_str(&format!("{} ",t.token_value)),
				}
			}
		}
		rtn
	}

	fn postfix_expression_text(&self) -> String{
		let mut rtn = String::new();
		for t in &self.postfix_expression{
			//	if the token is a string println it in quotes
			if t.token_type == TokenType::STRING{
				rtn.push_str(&format!("\"{}\" ",t.token_value));
			}else{
				// if the token is a function call, println it with the argument count
				// otherwise just println it
				match t.token_type{
					TokenType::FUNCTION_CALL(_) => rtn.push_str(&format!("{} ", t.token_value)),
					TokenType::IDADDR => rtn.push_str(&format!("&{} ", t.token_value)),
					TokenType::NEW_COLLECTION => rtn.push_str("NEW_COLLECTION "),
					_ => rtn.push_str(&format!("{} ",t.token_value)),
				}
			}
		}
		rtn
	}

	// fn operators_text(&self) -> String{
	// 	let mut rtn = String::new();
	// 	for t in operators.iter().rev(){
	// 		rtn.push_str(
	// 			&format!("'{}'({}) ", &t.token_value, &self.token_prec(&t))
	// 		);
	// 	}
	// 	rtn
	// }

}
