use std::fmt;
use std::collections::HashMap;
use cli::*;
use tokenizer::*;
use model::*;
use itop::*;
use codeframe::*;
use macrolib::*;

use std::cell::RefCell;
use names::*;


#[derive(PartialEq, Eq, Clone, Hash)]
pub enum ParserState{
	Nothing,				// a dummy return
	Error(String),			// if an action encounters an error it returns this
	Program,
	Function,
	FunctionBody,
	FunctionSignature,
	FunctionParameter,
	Constant,
	Statement,
	Block,

	ForeachTarget,			//  Foreach syntax is odd so we have special cases here
	ForeachSource,
	ForeachBlock,

	IncDec,					// yet another special case
	IncDecStatement,		// Arrrrgghhhh!
	DictLiteral,			// to detect the beginning of dictionary literal
	Assignment,
	//AssignmentOp,
	Expression,
	ExpressionTerm,
	ExpressionFactor,

	FunctionCallArg,		// after recognizing a function call factor/term

	IdAddress,				// used when &<ID> is recognized

	Struct,
	StructBody,
	StructMember,
	StructInit,
	StructInstantiate,		// used when <var> = new <struct>

	Literal,
	GlobalLiteral,
	GlobalLiteralValue,
	LocalLiteral,
	LocalLiteralValue,

}
impl fmt::Display for ParserState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       	match self {
			ParserState::Nothing					=> write!(f,"Nothing"),
			ParserState::Error(msg)					=> write!(f,"Error! {}",msg),
			ParserState::Program					=> write!(f,"Program"),
			ParserState::Function					=> write!(f,"Function"),
			ParserState::FunctionBody				=> write!(f,"FunctionBody"),
			ParserState::FunctionSignature			=> write!(f,"FunctionSignature"),
			ParserState::FunctionParameter			=> write!(f,"FunctionParameter"),
			ParserState::Constant					=> write!(f,"Constant"),
			ParserState::Statement					=> write!(f,"Statement"),
			ParserState::Block						=> write!(f,"Block"),

			ParserState::ForeachTarget				=> write!(f,"ForeachTarget"),
			ParserState::ForeachSource				=> write!(f,"ForeachSource"),
			ParserState::ForeachBlock				=> write!(f,"ForeachBlock"),
		
			ParserState::IncDec						=> write!(f,"Inc/Dec"),
			ParserState::IncDecStatement			=> write!(f,"Inc/Dec Statement"),
			ParserState::DictLiteral				=> write!(f,"DictLiteral"),
			ParserState::Assignment					=> write!(f,"Assignment"),
			// ParserState::AssignmentOp				=> write!(f,"AssignmentOp"),
			ParserState::Expression					=> write!(f,"Expression"),
			ParserState::ExpressionTerm				=> write!(f,"ExpressionTerm"),
			ParserState::ExpressionFactor			=> write!(f,"ExpressionFactor"),
			ParserState::FunctionCallArg			=> write!(f,"FunctionCallArg"),

			ParserState::IdAddress					=> write!(f, "IdAddress"),

			ParserState::Struct						=> write!(f, "Struct"),
			ParserState::StructBody					=> write!(f, "StructBody"),
			ParserState::StructMember				=> write!(f, "StructMember"),
			ParserState::StructInit					=> write!(f, "StructInit"),
			ParserState::StructInstantiate			=> write!(f, "StructInstantiate"),

			ParserState::Literal					=> write!(f, "Literal"),
			ParserState::GlobalLiteral				=> write!(f, "GlobalLiteral"),
			ParserState::GlobalLiteralValue			=> write!(f, "GlobalLiteralValue"),
			ParserState::LocalLiteral				=> write!(f, "LocalLiteral"),
			ParserState::LocalLiteralValue			=> write!(f, "LocalLiteralValue"),
		}
	}
}

//	ParserContext is used to help actions make decsions about what to do next
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ParserContext{
	Nothing,
	Loop,
	CollectionSource,
	ArrayLiteral,
	DictLiteral,
	Literal,				// Literal <id> = <value>
	Assignment,				// <factor> <assignment operator> <expression>;
	IndexedTarget,			// <factor>[<index expression>] = <expression>;
	Constant,				// const <id> <assignment operator> expression>;
	Simple,					// print, return, etc.
	If,
	Else,
	While,
	Foreach,
	Eval,
	When,
	Otherwise,
	FunctionCallStatement,
	FunctionCallTerm,
	Struct,
	StructMember,
}

impl fmt::Display for ParserContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       	match self {
			ParserContext::Nothing					=> write!(f,"Nothing"),
			ParserContext::Loop						=> write!(f,"Loop"),
			ParserContext::CollectionSource			=> write!(f,"CollectionSource"),
			ParserContext::ArrayLiteral				=> write!(f,"ArrayLiteral"),
			ParserContext::DictLiteral				=> write!(f,"DictLiteral"),
			ParserContext::Literal					=> write!(f,"Literal"),
			ParserContext::Assignment				=> write!(f,"Assignment"),
			ParserContext::IndexedTarget			=> write!(f,"IndexedTarget"),
			ParserContext::Constant					=> write!(f,"Constant"),
			ParserContext::Simple					=> write!(f,"Simple"),
			ParserContext::If						=> write!(f,"If"),
			ParserContext::Else						=> write!(f,"Else"),
			ParserContext::While					=> write!(f,"While"),
			ParserContext::Eval						=> write!(f,"Eval"),
			ParserContext::When						=> write!(f,"When"),
			ParserContext::Otherwise				=> write!(f,"Otherwise"),
			ParserContext::Foreach					=> write!(f,"Foreach"),
			ParserContext::FunctionCallStatement	=> write!(f,"FunctionCallStatement"),
			ParserContext::FunctionCallTerm			=> write!(f,"FunctionCallTerm"),
			ParserContext::Struct					=> write!(f,"Struct"),
			ParserContext::StructMember				=> write!(f,"StructMember"),
		}
	}
}

#[derive(PartialEq, Eq, Clone, Hash)]
struct ParserStateTransitionKey{
	parser_state : ParserState,
	trigger : TokenCategory,
}
impl ParserStateTransitionKey{
	fn new(parser_state : ParserState, trigger : TokenCategory) -> ParserStateTransitionKey{
		ParserStateTransitionKey{
			parser_state:parser_state,
			trigger:trigger,
		}
	}
}

impl fmt::Display for ParserStateTransitionKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{},{}", self.parser_state, self.trigger)
	}
}

#[derive(PartialEq, Clone)]
struct ParserStateTransitionContent<'a>{

	//  New state
	new_parser_state : ParserState,

	//	If true then don't call the action
	do_nothing : bool,

	//	This is the "name" of the action
	action_name : String,

	//  The action function returns either None or Some(ParserState) which
	//	overrides the new_parser_state value in the content.  In some cases
	//	it will update the parser_context attribute, add a token to the
	//	expresion_list and call an "add" function in the model
	action : fn(&mut Parser<'a>) -> Option<ParserState>,
}

impl <'a> ParserStateTransitionContent<'a> {
	fn new(	  new_parser_state : ParserState
			, do_nothing : bool
			, action_name : &str
			, action : fn(&mut Parser<'a>) -> Option<ParserState>) -> ParserStateTransitionContent <'a>{
		ParserStateTransitionContent{
			new_parser_state : new_parser_state,
			do_nothing : do_nothing,		// if true don't call action function
			action_name : action_name.to_string(),
			action : action,
		}
	}

	fn new_initial() -> ParserStateTransitionContent <'a>{
		ParserStateTransitionContent{
			new_parser_state : ParserState::Nothing,
			do_nothing : true,
			action_name : "do_nothing".to_string(),
			action : Parser::do_nothing,
		}
	}
}

impl<'a> fmt::Display for ParserStateTransitionContent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} {}", self.do_nothing, self.new_parser_state)
	}
}

pub struct Parser<'a>{
	cli : & 'a CLI<'a>,
	pub model : Model<'a>,
	tokenizer : Tokenizer<'a>,
	token : Token,
	pub current_state : ParserState,
	parser_context : Vec<ParserContext>,
	current_transition : ParserStateTransitionContent<'a>,

	//	Some tempoary storage and processing variables
	brace_counter : i32,
	//bracket_counter : i32,

	/***************************************************************************************/
	//	these are used to hack and backpatch function calls
	//	in expressions
	paren_counter : i32,  // general paren counter (may not need it)
	function_call_paren_counters : Vec<i32>,	// counters specifically for one function call
	comma_counters : Vec<i32>,		//  to figure out how many arguments a function call had
	function_call_locations : Vec<usize>,	// for the backpatching operations
	/***************************************************************************************/

	statement_list_item_counter : i32,		// used to count commas in non-function call contexts

	if_model_context : Vec<usize>,
	eval_model_context : Vec<usize>,

	//	The struct header index is where in the struct header we find the index
	//	of a top level struct in the structs table
	struct_header_ix : Vec<usize>,

	//	The struct index is where in the structs table a struct or child is
	//	located
	struct_ix : Vec<usize>,
	struct_member_name : String,

	simple : Token,				// PRINT, EPRINT, RETURN, ETC.

	//	The value is a list because eventually we'll want to support
	//	expressions, array literals, and dictionary literals.
	literal_id : Token,			// literal <id>
	literal_value : Vec<Token>,	// literal <id> = <literal expression>

	//	When we see a function call in an expression, we save it's name and the parameter count
	//expression_function_call_name: Vec<Token>,

	function_name : Token,
	function_parameters : Vec<String>,
	function_entry_flag : bool,

	statement_factor : Token,
	assignment_target : Token,
	assignment_target_index_expression : Vec<Token>,
	assignment_operator : Token,

	foreach_target : Token,					// the target of the foreach
	foreach_source_type : TokenType,		// the type of source ID, LBRACKET, LBRACE				// if
	foreach_source : Vec<Token>,			// the source token list; if ID only one element

	eval_brace_counter : Vec<Vec<usize>>,	// it's a stack of stacks to enable eval within eval

	//  all of the tokens caputred that make up an expression
	infix_expression : Vec<Token>,

	//	where the converted infix to postfix expression tokens end up
	postfix_expression : Vec<Token>,

	parser_state_transition_table : HashMap<ParserStateTransitionKey,ParserStateTransitionContent<'a>>,
}

impl <'a> Parser<'a>{
	pub fn new(cli : & 'a CLI<'a>, names : & 'a RefCell<Names>) -> Parser<'a>{
		Parser{
			cli : cli,
			model : Model::new(cli, names),
			tokenizer : Tokenizer::new(cli),
			token : Token::new(),
			current_transition : ParserStateTransitionContent::new_initial(),

			brace_counter : 0,
			//bracket_counter : 0,

			/***************************************************************************************/
			paren_counter : 0,
			function_call_paren_counters : Vec::new(),
			comma_counters : Vec::new(),
			function_call_locations : Vec::new(),
			/***************************************************************************************/

			statement_list_item_counter : -1,

			if_model_context : Vec::new(),
			eval_model_context : Vec::new(),

			struct_header_ix : Vec::new(),
			struct_ix : Vec::new(),
			struct_member_name : String::new(),

			simple :Token::new(),

			literal_id : Token::new(),			// literal <id>
			literal_value : Vec::new(),			// litera <id> = <literal expression>

			function_name : Token::new(),
			function_parameters : Vec::new(),
			function_entry_flag : false,
			
			statement_factor : Token::new(),
			assignment_target : Token::new(),
			assignment_target_index_expression : Vec::new(),
			assignment_operator : Token::new(),

			foreach_target : Token::new(),					// the target of the foreach
			foreach_source_type : TokenType::NONE,					// the type of source ID, LBRACKET, LBRACE				// if
			foreach_source : Vec::new(),							// the source token list; if ID only one element

			eval_brace_counter : Vec::new(),

			current_state : ParserState::Program,
			parser_context: vec![ParserContext::Nothing],

			infix_expression : Vec::new(),
		
			postfix_expression : Vec::new(),

			parser_state_transition_table : HashMap::from([
				//
				//	Each table entry is:
				//		- key (consisting of state + category of token just read)
				//		- A look-ahead token type (which might be NONE, which means nothing to check)
				//		- A flag contolling whether or not to call the action. if flase, then the action is pass and it is not executed
				//		- The next state to go to
				//		- An action (which may or may not be executed)
				//
				//	NOTE:  do not remove the $$ comments as they are used by the "parsergen" program to
				//	insert transition_table content.
				//$$FOOBAR$$
//BEGIN INSERT
			(ParserStateTransitionKey::new(ParserState::Program,TokenCategory::FunctionDeclaration),
				ParserStateTransitionContent::new(ParserState::Function, false, "function_declaration", Parser::function_declaration)),
			(ParserStateTransitionKey::new(ParserState::Function,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::FunctionSignature, false, "function_name", Parser::function_name)),
			(ParserStateTransitionKey::new(ParserState::FunctionSignature,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::FunctionParameter, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::FunctionSignature,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Statement, false, "function_no_parameters", Parser::function_no_parameters)),
			(ParserStateTransitionKey::new(ParserState::FunctionParameter,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::FunctionParameter, false, "function_parameter", Parser::function_parameter)),
			(ParserStateTransitionKey::new(ParserState::FunctionParameter,TokenCategory::Comma),
				ParserStateTransitionContent::new(ParserState::FunctionParameter, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::FunctionParameter,TokenCategory::RParen),
				ParserStateTransitionContent::new(ParserState::Block, false, "function_parameter", Parser::function_parameter)),
			(ParserStateTransitionKey::new(ParserState::Program,TokenCategory::StructDeclaration),
				ParserStateTransitionContent::new(ParserState::Struct, false, "struct_declaration", Parser::struct_declaration)),
			(ParserStateTransitionKey::new(ParserState::Struct,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::Struct, false, "struct_name", Parser::struct_name)),
			(ParserStateTransitionKey::new(ParserState::Struct,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::StructBody, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::StructBody,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::StructMember, false, "struct_member", Parser::struct_member)),
			(ParserStateTransitionKey::new(ParserState::StructMember,TokenCategory::Semi),
				ParserStateTransitionContent::new(ParserState::StructBody, false, "struct_member_add", Parser::struct_member_add)),
			(ParserStateTransitionKey::new(ParserState::StructMember,TokenCategory::AssignmentOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "struct_member_init", Parser::struct_member_init)),
			(ParserStateTransitionKey::new(ParserState::StructBody,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::Program, false, "rbrace", Parser::rbrace)),
			(ParserStateTransitionKey::new(ParserState::StructInit,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::Program, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::StructBody,TokenCategory::StructDeclaration),
				ParserStateTransitionContent::new(ParserState::Struct, false, "struct_declaration", Parser::struct_declaration)),
			(ParserStateTransitionKey::new(ParserState::Program,TokenCategory::Literal),
				ParserStateTransitionContent::new(ParserState::GlobalLiteral, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::GlobalLiteral,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::GlobalLiteralValue, false, "literal_id", Parser::literal_id)),
			(ParserStateTransitionKey::new(ParserState::GlobalLiteralValue,TokenCategory::AssignmentOp),
				ParserStateTransitionContent::new(ParserState::GlobalLiteralValue, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::GlobalLiteralValue,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::GlobalLiteralValue, false, "literal_value", Parser::literal_value)),
			(ParserStateTransitionKey::new(ParserState::GlobalLiteralValue,TokenCategory::Semi),
				ParserStateTransitionContent::new(ParserState::Program, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::Block,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Statement, false, "begin_block", Parser::begin_block)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::Assignment, false, "statement_factor", Parser::statement_factor)),
			(ParserStateTransitionKey::new(ParserState::Assignment,TokenCategory::AssignmentOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "assignment_op", Parser::assignment_op)),
			(ParserStateTransitionKey::new(ParserState::StructInstantiate,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::Statement, false, "struct_instantiate", Parser::struct_instantiate)),
			(ParserStateTransitionKey::new(ParserState::Assignment,TokenCategory::LBracket),
				ParserStateTransitionContent::new(ParserState::Expression, false, "indexed_target", Parser::indexed_target)),
			(ParserStateTransitionKey::new(ParserState::Assignment,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::Expression, false, "function_call_statement", Parser::function_call_statement)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Verb),
				ParserStateTransitionContent::new(ParserState::Expression, false, "verb", Parser::verb)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Else),
				ParserStateTransitionContent::new(ParserState::Block, false, "keyword_else", Parser::keyword_else)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Loop),
				ParserStateTransitionContent::new(ParserState::Block, false, "keyword_loop", Parser::keyword_loop)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Otherwise),
				ParserStateTransitionContent::new(ParserState::Block, false, "keyword_otherwise", Parser::keyword_otherwise)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Semi),
				ParserStateTransitionContent::new(ParserState::Statement, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Literal),
				ParserStateTransitionContent::new(ParserState::LocalLiteral, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::LocalLiteral,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::LocalLiteralValue, false, "literal_id", Parser::literal_id)),
			(ParserStateTransitionKey::new(ParserState::LocalLiteralValue,TokenCategory::AssignmentOp),
				ParserStateTransitionContent::new(ParserState::LocalLiteralValue, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::LocalLiteralValue,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::LocalLiteralValue, false, "literal_value", Parser::literal_value)),
			(ParserStateTransitionKey::new(ParserState::LocalLiteralValue,TokenCategory::Semi),
				ParserStateTransitionContent::new(ParserState::Statement, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::Foreach),
				ParserStateTransitionContent::new(ParserState::ForeachTarget, false, "foreach_init", Parser::foreach_init)),
			(ParserStateTransitionKey::new(ParserState::ForeachTarget,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::ForeachSource, false, "foreach_target", Parser::foreach_target)),
			(ParserStateTransitionKey::new(ParserState::ForeachSource,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::ForeachBlock, false, "foreach_source", Parser::foreach_source)),
			(ParserStateTransitionKey::new(ParserState::ForeachSource,TokenCategory::LBracket),
				ParserStateTransitionContent::new(ParserState::Expression, false, "foreach_source", Parser::foreach_source)),
			(ParserStateTransitionKey::new(ParserState::ForeachSource,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Expression, false, "foreach_source", Parser::foreach_source)),
			(ParserStateTransitionKey::new(ParserState::ForeachBlock,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Statement, true, "do_nothing", Parser::do_nothing)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::BinaryOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "unary_op_test", Parser::unary_op_test)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::UnaryOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "unary_op_test", Parser::unary_op_test)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::IncDec),
				ParserStateTransitionContent::new(ParserState::Expression, false, "unary_op_test", Parser::unary_op_test)),
			(ParserStateTransitionKey::new(ParserState::IdAddress,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "id_address", Parser::id_address)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::Factor),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "expression_factor", Parser::expression_factor)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::Expression, false, "lparen", Parser::lparen)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::LBracket),
				ParserStateTransitionContent::new(ParserState::Expression, false, "array_literal", Parser::array_literal)),
			(ParserStateTransitionKey::new(ParserState::FunctionCallArg,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "function_call_argument", Parser::function_call_argument)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::Semi),
				ParserStateTransitionContent::new(ParserState::Statement, false, "statement_end", Parser::statement_end)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::Comma),
				ParserStateTransitionContent::new(ParserState::Expression, false, "expression_list_item", Parser::expression_list_item)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::RParen),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "rparen", Parser::rparen)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Statement, false, "lbrace", Parser::lbrace)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::BinaryOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "binary_op", Parser::binary_op)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::RelOp),
				ParserStateTransitionContent::new(ParserState::Expression, false, "binary_op", Parser::binary_op)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::IncDec),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "post_inc_dec", Parser::post_inc_dec)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::Expression, false, "lparen", Parser::lparen)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::LBracket),
				ParserStateTransitionContent::new(ParserState::Expression, false, "index_expression", Parser::index_expression)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::RBracket),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "rbracket", Parser::rbracket)),
			(ParserStateTransitionKey::new(ParserState::Expression,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::DictLiteral, false, "dict_literal_begin", Parser::dict_literal_begin)),
			(ParserStateTransitionKey::new(ParserState::DictLiteral,TokenCategory::LBrace),
				ParserStateTransitionContent::new(ParserState::Expression, false, "dict_literal_kv_begin", Parser::dict_literal_kv_begin)),
			(ParserStateTransitionKey::new(ParserState::DictLiteral,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "dict_literal_end", Parser::dict_literal_end)),
			(ParserStateTransitionKey::new(ParserState::ExpressionFactor,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::ExpressionTerm, false, "dict_literal_kv_end", Parser::dict_literal_kv_end)),
			(ParserStateTransitionKey::new(ParserState::ExpressionTerm,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::DictLiteral, false, "dict_literal_kv_end", Parser::dict_literal_kv_end)),
			(ParserStateTransitionKey::new(ParserState::DictLiteral,TokenCategory::Comma),
				ParserStateTransitionContent::new(ParserState::DictLiteral, false, "expression_list_item", Parser::expression_list_item)),
			(ParserStateTransitionKey::new(ParserState::ExpressionFactor,TokenCategory::LParen),
				ParserStateTransitionContent::new(ParserState::ExpressionFactor, false, "expression_term", Parser::expression_term)),
			(ParserStateTransitionKey::new(ParserState::ExpressionFactor,TokenCategory::LBracket),
				ParserStateTransitionContent::new(ParserState::Expression, false, "array_literal", Parser::array_literal)),
			(ParserStateTransitionKey::new(ParserState::Statement,TokenCategory::RBrace),
				ParserStateTransitionContent::new(ParserState::Statement, false, "rbrace", Parser::rbrace)),
//END INSERT
				//$$ENDFOOBAR$$
			])
		}
	}

	/*************************************************************
	***  Parser State Machine
	*************************************************************/

	fn _eval_model_context_text(&self) -> String{
		if self.eval_model_context.len() == 0{return "NONE".to_string()}

		let mut rtn = String::new();
		let mut count = 0;
		for c in &self.eval_model_context{
			if count > 0{
				rtn.push_str(", ");
			}
			rtn.push_str(&format!("{}",c));
			count += 1;
		}

		rtn.to_string()
	}

	fn _parser_context_text(&self) -> String{
		let mut rtn = String::new();
		let mut count = 0;
		for c in &self.parser_context{
			if count > 0{
				rtn.push_str(", ");
			}
			rtn.push_str(&format!("{}",c));
			count += 1;
		}

		rtn.to_string()
	}

	//	The "args" parameter contains a reference to a vector containing
	//	all of the non-switch arguments (except for the name of the CPL program itself)
	pub fn parse_and_gen (& 'a mut self) -> (bool,Option<FrameMap>){
		// make sure all of the expression tokens go to the right
		// place initially.
		self.token = self.next_token();
		let mut error_flag = false;

		while self.token.token_type != TokenType::EOF{				
			//	fetch the next transition state
			match self.fetch_transition(){
				Err(msg) => {
					eprintln!("{msg}");
					return (false,None);
				},
				Ok(transition) => {
					self.current_transition = transition.clone();
				}
			}

			//  *******************************************************************************************
			//  NOTE:  do not change self.current_state before calling the action since many actions
			//	need to know that current state when they are called.
			//  *******************************************************************************************

			if self.cli.is_debug_bit(TRACE_PARSE_LOOP){
				eprintln!("transition: {} + {} (\"{}\")-> {} : \"{}\" [{}]"
				, self.current_state
				, self.token.token_category
				, self.token.token_value
				, self.current_transition.new_parser_state
				, self.current_transition.action_name
				, self.token.line_text
				);
			}

			//	If the do_nothing flag is false (meaning "do_someting") then
			//	call the action and, if the action returns a new state, overrid
			//	the parser state from the transiion table
			if !self.current_transition.do_nothing{
				match (self.current_transition.action)(self){
					None => self.current_state = self.current_transition.new_parser_state.clone(),
					Some(p) => match p{
						ParserState::Error(m) => {
							eprintln!("{} from CPL source line {} -- ({})",m, self.token.line_number, self.token.line_text);
							error_flag = true;
							break;
						},
						_ => {
							self.current_state = p;
						}
					},
				}
			}else{
				self.current_state = self.current_transition.new_parser_state.clone();
			}

			if self.cli.is_debug_bit(TRACE_PARSE_LOOP){eprintln!("    final state: {} context: {}", self.current_state, self.parser_context.last().unwrap());}
			self.token = self.next_token();	
		}

		if self.brace_counter > 0{
			abend!(format!("Imbalanced begin/end braces at EOF - brace count={}", self.brace_counter));
		}

		if error_flag{
			return (false, None);
		}

		if self.cli.is_debug_bit(DUMP_STRUCTS){self.model.dump_structs();}

		self.model.generate_code_from_model();

		if self.cli.is_debug_bit(DUMP_GEN_CODE){self.model.code_dump();}

		let frames = self.model.get_frames();
		(true, Some(frames))
		//self.model.exec();
		//true
	}
	
	//	Returns a Result containing either a transition or an error message
	fn fetch_transition(&mut self) -> Result<&ParserStateTransitionContent<'a>, String>{
		let key = ParserStateTransitionKey::new(self.current_state.clone(), self.token.token_category);
		match &self.parser_state_transition_table.get(&key){
			None => return Err(self.syntax_error_text(&key,self.token.clone())),
			Some(s) => return Ok(s),
		}
	}
	fn syntax_error_text(&self, input_key : &ParserStateTransitionKey, token : Token) -> String{
		let mut error_text = String::from(format!("Syntax Error. Key=<{}> {} Expecting one of: ", input_key, token.token_value));
		for key in self.parser_state_transition_table.keys(){
			if key.parser_state == input_key.parser_state{
				error_text.push_str(&format!(" {}",key.trigger));
			}
		}

		if self.token.token_category == TokenCategory::Factor{
			let prev_token = self.tokenizer.prev_token(2);
			error_text.push_str (&format!(":  It looks like you're missing a semi-colon on previous line {}: \"{}\"",prev_token.line_number, prev_token.line_text));
		}else{
			error_text.push_str(&format!(": Got \"{}\" at line {}", self.token.token_value, self.token.line_number));
		}
		error_text.clone()
	}


	/*************************************************************
	***  Parser Helpers
	*************************************************************/
	fn increment_eval_brace_counter(&mut self){
		let stack_ix = self.eval_brace_counter.len() - 1;
		let counter_ix = self.eval_brace_counter[stack_ix].len() - 1;
		self.eval_brace_counter[stack_ix][counter_ix] += 1;
	}

	fn decrement_eval_brace_counter(&mut self) -> usize{
		let stack_ix = self.eval_brace_counter.len() - 1;
		let counter_ix = self.eval_brace_counter[stack_ix].len() - 1;
		self.eval_brace_counter[stack_ix][counter_ix] -= 1;
		return self.eval_brace_counter[stack_ix][counter_ix];
	}

	fn new_eval_brace_counter(&mut self){
		let bc = vec![0];
		self.eval_brace_counter.push(bc.clone());
	}

	fn remove_eval_brace_counter(&mut self){
		self.eval_brace_counter.pop();
	}

	fn _token_list_text(&self, tokens : &Vec<Token>) -> String{
		let mut display = String::new();
		for t in tokens{
			display.push_str(" ");
			display.push_str(&t.token_value);
		}
		display
	}


	/*************************************************************
	***  Parser Actions
	*************************************************************/
	//
	//	A Parser Action does some checking and returns true if
	//	there are no problems.  false if there is an issue (after printing an error message)

	//	Do nothing.  This is a dummy routine that is required by the
	//	signature of state transition content but it is never called
	fn do_nothing (&mut self) -> Option<ParserState>{None}


	//	&<ID> has been recognized so instead of pushing ID onto the
	//  the expression we push IDADDR
	fn id_address(&mut self) -> Option<ParserState>{
		if self.token.token_type != TokenType::ID{
			abend!(format!("from id_adress:  Houston we have a problem.  Token should have been ID but was {}",self.token.token_type));
		}
		let mut id_addr = self.token.clone();
		id_addr.token_type = TokenType::IDADDR;
		self.infix_expression.push(id_addr.clone());
		None
	}

	//	Called from state Expression, with a binary op, unary op or incdec
	//	NOTE:  <expression> can NEVER start with a <binary op> that is an
	//	actual binary operator.  <binary op> can only follow a factor or )
	//	or ] or } (but that's another story)
	fn unary_op_test (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: unary_op_test {}", self.token.token_type)}

		match self.token.token_type{
			TokenType::SUB 		|
			TokenType::ADD 		| 
			TokenType::DAMNIT 	=>{
				if !self.look_ahead_test_multiple(&[TokenType::INTEGER, TokenType::FLOAT, TokenType::ID, TokenType::LPAREN].to_vec()){
					return Some(ParserState::Error(format!("Syntax error from unary_op_test, line {}: Op can only predede an ID or Number",line!())));
				}
				self.infix_expression.push(self.binary_op_to_unary_op());
			},
			TokenType::MM		|
			TokenType::PP		|
			TokenType::LENGTH_OF =>{
				if !self.look_ahead_test_token(TokenType::ID){
					return Some(ParserState::Error(format!("Syntax error from unary_op_test, line {}: inc/dec can only predede an ID or function call",line!())));
				}
				self.infix_expression.push(self.token.clone());
			},

			//	If we see a BITWISE_AND followed by ID then the program intends this to be
			//	a reference to the ID and so it needs to become IDADDR
			TokenType::BITWISE_AND =>{
				if self.look_ahead_test_token(TokenType::ID){
					return Some(ParserState::IdAddress);
				}
			}
			_ => return Some(ParserState::Error("Syntax Error: Only '-' or '+' or '!".to_string())),		// bad!
		}
		None
	}


	//	This is called when a factor is preceded by a "++" or "--".  the look ahead must be an ID
	//	or this function fails.  Get the previous token to figure if it's a ++ or --
	// fn pre_inc_dec(&mut self) -> Option<ParserState>{
	// 	if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: pre_inc_dec {}", self.token.token_type)}

	// 	//	The current token must be an ID or an error occurs
	// 	match self.token.token_type{
	// 		TokenType::ID => {
	// 			let back_token = self.prev_token();

	// 			self.infix_expression.push(back_token.clone());
	// 			self.infix_expression.push(self.token.clone());

	// 			if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: pre_inc_dec is {}", back_token.token_value)}
	// 			return Some(self.current_transition.new_parser_state.clone());
	// 		},
	// 		_ => return Some(ParserState::Error(format!"Syntax error from pre_inc_dec, line {}: Inc/Dec can only predede an ID", line!()))),
	// 	}
	// }


	//	This is called when inc/dec follows a factor (ID or function call)
	fn post_inc_dec(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: post_inc_dec \"{}\"", self.token.token_value)}
		self.infix_expression.push(self.token.clone());
		None
	}

	//	<function> :: fn | entry | entry fn
	fn function_declaration(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_declaration {} context={:?}", self.token.token_type, self.parser_context)}

		//  if we see ENTRY then we might see FN but that is just suger and can be ignored
		if self.token.token_type == TokenType::ENTRY{
			if self.look_ahead_test_token(TokenType::FN){
				self.next_token();
			}
			self.function_entry_flag = true;
		}else{
			self.function_entry_flag = false;
		}

		None
	}
	//	<function> :: fn | entry | entry fn <id>
	fn function_name(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_name {}", self.token.token_type)}
		self.function_name = self.token.clone();
		self.function_parameters.clear();
		None
	}

	//	<function> :: fn | entry | entry fn <id> (
	// fn function_signature(&mut self) -> Option<ParserState>{
	// 	if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_signature {}", self.token.token_type)}
	// 	None
	// }

	//	<function> :: fn | entry | entry fn <id> (<parameter,....)
	fn function_parameter(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_parameter \"{}\"", self.token.token_value)}

		//	if we're all done collecting parameters, then add the function declaration to the model
		if self.token.token_type == TokenType::RPAREN{
			self.model.add_function(self.function_name.token_value.clone(), self.function_entry_flag, self.function_parameters.clone(), &self.cli.cl_args);
		}else{
			self.function_parameters.push(self.token.token_value.clone());
		}

		None
	}

	//	A funciton can be declared with not parameters by simply leaving off
	//	the parameter block "(...)";  Here we've seen:  entry/fn foo{
	fn function_no_parameters(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_no_parameters \"{}\" parser_context:{}", self.token.token_value,self.parser_context.last().unwrap())}
		self.model.add_function(self.function_name.token_value.clone(), self.function_entry_flag, self.function_parameters.clone(), &self.cli.cl_args);
		self.begin_block();
		None
	}

	fn struct_declaration(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: struct_declaration \"{}\" parser_context:{}", self.token.token_value,self.parser_context.last().unwrap())}
		self.parser_context.push(ParserContext::Struct);
		None
	}


	fn struct_name(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: struct_name \"{}\" parser_context:{}", self.token.token_value,self.parser_context.last().unwrap())}
		//	When brace_counter is 0, it means that the parser has just recognized the beginning of a
		//	top-level struct.  When the brace_counter is > 0, it means that the parser has just
		//	recognized the beginnig of a substruct.
		if self.brace_counter == 0{
			//	This is the start of a top level struct.  We need to save the location in the
			//	struct_header_table.
			let struct_context = self.model.add_struct(&self.token.token_value);
			self.struct_header_ix.push(struct_context.0);
			self.struct_ix.push(struct_context.1);
		}else{
			//	Since the brace_counter is > 0, we are creating a child struct.  We pass the header index and
			//	the struct index is returned.  We also pass the "level" which is how we'll know how to find
			//	the parent struct (which is always at level - 1)
			//let level = self.brace_counter as usize + 1;
			let child_ix= self.model.add_child_struct(&self.token.token_value, *self.struct_ix.last().unwrap());
			self.struct_ix.push(child_ix);
		}

		self.brace_counter += 1;
		None
	}
	fn struct_member(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: struct_member \"{}\" parser context={:?}", self.token.token_value,self.parser_context)}
		self.struct_member_name = self.token.token_value.clone();
		None
	}

	fn struct_member_add(&mut self) -> Option<ParserState>{
		//	This is called when there IS NOT an initializer.  If there is an initializer, the function
		//  dispatch_model_adder is called when the ";" that terminates the initializer is recognized
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: struct_member_add \"{}\" model_context:{} parser_context:{} braces={}", self.struct_member_name, self.struct_ix.last().unwrap(),self.parser_context.last().unwrap(), self.brace_counter);}
		self.postfix_expression.clear();
		self.model.add_struct_member(&self.struct_member_name,&self.postfix_expression, *self.struct_ix.last().unwrap());
		self.infix_expression.clear();
		None
	}

	fn struct_member_init(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: struct_member_init \"{}\"", self.token.token_value)}
		self.parser_context.push(ParserContext::StructMember);
		self.infix_expression.clear();
		None
	}

	//	we get to here after we've seen <target>=new. This had better be an id.  If it is, we
	//	look ahead to make sure we see a ";" because the grammar here is precise and simple:
	//
	//		<target> = new <struct name>;
	//
	//	as noted elsewhere, someday we may want to do:
	//
	//		<target> = new <string name>(<member initialization list>);
	//		<<member initialization list> ::= <init item> [,<member initialization list>]
	//		<init item> ::= <member name>:<initial value>
	//
	//	but at the moment that's a bridge too far
	//
	//	set the ID type to Struct and add the asssignment statement
	fn struct_instantiate(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {
			eprintln!("    Action: struct_instantiation \"{}\"", self.token.token_value);
			self.model.dump_structs();
		}
	
		if self.token.token_type != TokenType::ID {
			return Some(ParserState::Error(format!("Syntax Error:  Expecting to see an <ID> following the assignment operator. Found:{}",self.token.token_type)));
		}
		if !self.look_ahead_test_token(TokenType::SEMI){
			let local_token = self.next_token();
			return Some(ParserState::Error(format!("Syntax Error:  Expecting to see a ';' following struct name. Found:{}",local_token.token_value)));
		}
		//	eat the ';'
		self.next_token();

		self.token.token_type = TokenType::STRUCT;
		self.token.token_category = TokenCategory::StructDeclaration;
		self.infix_expression.clear();
		self.infix_expression.push(self.token.clone());

		//	if we've seen <id> = new <id> then we know we're instantiating a
		//	struct and there's nothing left to do.
		self.model.add_struct_instantiate(&self.assignment_target, &self.token);
		None
	}


	//	any time we see a "{"
	fn begin_block(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: begin_block \"{}\" context={:?} braces={}", self.token.token_value, self.parser_context, self.brace_counter);}
		self.brace_counter += 1;
		None
	}

	// fn collection_target (&mut self) -> Option<ParserState>{
	// 	if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: collection_target \"{}\"", self.token.token_value)}
	// 	None
	// }

	fn statement_factor(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: statement_factor \"{}\" context={:?}", self.token.token_value, self.parser_context);}
		self.statement_factor = self.token.clone();
		//  initialize some things
		self.assignment_target_index_expression.clear();
		None
	}

	fn do_infix_to_postfix(&mut self){
		//	get the reference to i_to_p conversion functions
		let mut itop = InfixToPostfix::new(self.cli);

		//	ensure the result is empty
		self.postfix_expression.clear();

		//	get the converted infix expression and update the postfix
		self.postfix_expression.append(itop.infix_to_postfix(&self.infix_expression));
	}
	
	//	This adds a statement to the model depending on the context
	fn dispatch_model_adder(&mut self, parser_context : ParserContext){
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("        dispatch_model_adder context={}", parser_context)}

		//	Now, add whatever we found
		match parser_context{
			ParserContext::Assignment => {
				//eprintln!(".... dispatch_model_adder\nassignment_target ({})\nstatement_factor ({})", self.assignment_target, self.statement_factor);
				self.do_infix_to_postfix();
				self.model.add_assignment_statement(self.assignment_target.clone(), self.assignment_operator.clone(), &self.assignment_target_index_expression, &self.postfix_expression.clone());
			},
			ParserContext::Simple => {
				self.do_infix_to_postfix();
				self.model.add_simple_statement(self.simple.clone(),  self.postfix_expression.clone());
			},
			ParserContext::FunctionCallStatement => {
				//	If we've decided that the factor of the assignment statement is, in fact, a function name
				//	send it to the model
				self.do_infix_to_postfix();
				self.model.add_function_call_statement (self.statement_factor.clone(), self.postfix_expression.clone(), (self.statement_list_item_counter + 1) as usize);
			},

			ParserContext::CollectionSource => {
				let parser_context = self.parser_context.pop().unwrap().clone();
				self.dispatch_model_adder(parser_context);
			},

			ParserContext::StructMember => {
				self.do_infix_to_postfix();
				self.model.add_struct_member(&self.struct_member_name, &self.postfix_expression, *self.struct_ix.last().unwrap());
			}

			_=>{
				abend!(format!("from dispatch_model_adder.  Unknown parser_context: {}", parser_context));
			}
		}
	}

	fn statement_end(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {
			eprintln!("    Action: statement_end \"{}\" context={:?} braces={}", self.token.token_value, self.parser_context, self.brace_counter);
		}

		if self.token.token_type != TokenType::SEMI{
			eprintln!("*********** token is other {}", self.token.token_type);
			return None;
		}

		//	There must be a context so w e can figure out what to add to the model
		if *self.parser_context.last().unwrap() == ParserContext::Nothing {
			abend!(format!("from Action statement_end:  Houston we have a problem.  Expecting a current context but found only Nothing"));
		}

		//	get the current parser_context and pop it
		let parser_context = self.parser_context.pop().unwrap();

		self.dispatch_model_adder(parser_context.clone());

		self.infix_expression.clear();

		if parser_context == ParserContext::StructMember{
			return Some(ParserState::StructBody);
		}

		None
	}


	//	This is called when an assignment op is seen (e.g. "+=").  It also allows us to say that the
	//	previous token (assuming it was a factor), is a scalar target.  But we look ahead and if we
	//	see the keyword "new" and the op is '=' we know we're instantiating a struct
	fn assignment_op(&mut self) -> Option<ParserState>{
		//  The target MUST BE and ID
		match self.statement_factor.token_type{
			TokenType::ID					|
			TokenType::QUALIFIED_ID			|			// struct member reference
			TokenType::INDEXED_ID			|
			TokenType::INDEXED_QUALIFIED_ID	=>{},
			_ => return Some(ParserState::Error(format!("Syntax Error: Target of an assignment must be an ID or ID:QUAL or ID[<index>] or ID:QUAL[<index>]. We found: {} context={:?}", self.statement_factor.token_type, self.parser_context))),
		}

		//  Set the context
		self.parser_context.push(ParserContext::Assignment);


		//	Clear the infix expression
		self.infix_expression.clear();

		//	Save what we've found for passing to the model after the expression is completed
		self.assignment_target = self.statement_factor.clone();
		self.assignment_operator = self.token.clone();

		//	look ahead one token.  If we see "new" then we are instantiating a
		//	struct.  Eat the "new" because we don't actually need it any more

		if self.look_ahead_test_token(TokenType::NEW) && self.token.token_type == TokenType::ASG_EQ{
			self.statement_factor.token_type = TokenType::STRUCT;
			if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: assignment_op : struct instantiation \"{}\" target=new \"{}\"", self.token.token_value, self.assignment_target.token_value)}
			self.next_token();
			return Some(ParserState::StructInstantiate);
		}

		//	Otherwise it's just a "normal" assignment statement
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: assignment_op \"{}\" target=\"{}\"", self.token.token_value, self.assignment_target.token_value)}
		None
	}

	//	This is called when we've seen "<factor>(" within the context of a statement (i.e. on the left
	//	side of an assignment operator)
	fn function_call_statement(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_call_statement \"{}\" context={:?}", self.token.token_value, self.parser_context)}

		self.parser_context.push(ParserContext::FunctionCallStatement);

		//	We're looking at an LPAREN, set the global paren counter to 1
		self.paren_counter = 1;
		
		//	create a function call specific paren counter and set it
		//	to 1.
		self.function_call_paren_counters.push(1);

		//	clear the expression list
		self.infix_expression.clear();
		
		//	look ahead 1 token and if we see a  ")" then there are no
		//	arguments in this call and when that's the case, we initialize
		//	the item counter to -1 so that it'll be 0 when we add the statement
		//	to the model.  OTHERWISE we set it to 0 because we know there will be
		//	at least 1 argument.
		if self.look_ahead_test_token(TokenType::RPAREN){
			self.statement_list_item_counter = -1;
			return Some(ParserState::ExpressionTerm);
		}else{
			self.statement_list_item_counter = 0;
		}
		None
	}

	// fn function_call_term(&mut self) -> Option<ParserState>{
	// 	if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_call_term \"{}\" context={:?}", self.token.token_value, self.parser_context);}
	// 	None
	// }

	fn expression_term(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: expression_term \"{}\" context={:?}", self.token.token_value, self.parser_context)}
		self.infix_expression.push(self.token.clone());
		None
	}

	fn binary_op(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: binary_op \"{}\" context={:?}", self.token.token_value, self.parser_context)}
		self.infix_expression.push(self.token.clone());
		None
	}


	//	The first token of a sub-expression is a factor (e.g. ID, NUMBER, etc).  If it's an ID
	//	and the next token after that is a "(" then that factor is, in fact a function name
	//	this bit of the expression is a function call.  E.G.:
	//
	//		a = foo(x,y,z);
	//
	//	we don't add the function call to the infix expression here, rather we do it
	//	in the "function_call_argument" action
	fn expression_factor(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: expression_factor \"{}\" context={:?}", self.token.token_value, self.parser_context)}

		//  if this is an ID followed by a "(" then go to a special "argument" state
		//	otherwise go to wherever the transition table sends us.  If it's and ID followed by '['
		//	then change the token from ID to INDEXED_ID
		if self.token.token_type == TokenType::ID{

			if self.look_ahead_test_token(TokenType::LPAREN){
				return Some(ParserState::FunctionCallArg);
			}else if self.look_ahead_test_token(TokenType::LBRACKET){
				let mut indexed_id = self.token.clone();
				if indexed_id.token_type == TokenType::ID{
					indexed_id.token_type = TokenType::INDEXED_ID;
					indexed_id.token_category = TokenCategory::IndexedId;	
				}else if indexed_id.token_type == TokenType::QUALIFIED_ID{
					indexed_id.token_type = TokenType::INDEXED_QUALIFIED_ID;
					indexed_id.token_category = TokenCategory::IndexedId;	
				}
				self.infix_expression.push(indexed_id.clone());
			}else if self.token.token_value.find(':') != None{
				let mut struct_ref = self.token.clone();
				struct_ref.token_type = TokenType::QUALIFIED_ID;
				self.infix_expression.push(struct_ref.clone());
			}else{
				//	NOT a function call, OR indexed id save the input token (some kind fo factor)
				self.infix_expression.push(self.token.clone());
			}
		}else{
			//  Not an ID at all
			self.infix_expression.push(self.token.clone());
		}
		None
	}

	//	We've seen ID and this is the "(" following which indicates that the ID
	//	is a function and this is a function call.  And, we have not added the
	//	the function call to the infix expression yet. We do that here.
	fn function_call_argument (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: function_call_argument \"{}\"", self.token.token_value)}

		//	Increment the expression wide paren counter
		self.paren_counter += 1;

		//	build the function call token using the previous token which we know is an ID as a basis. We set the
		// 	argument count to 0 (we'll update that later) and use the configured category and precedence
		//	for function calls
		let mut function_call_token = self.tokenizer.prev_token(2).clone();
		function_call_token.token_type = TokenType::FUNCTION_CALL(0);
		function_call_token.token_category = self.tokenizer.categories.get(&TokenType::FUNCTION_CALL(0)).unwrap().clone();
		//function_call_token.token_precedence = *self.tokenizer.in_stack_precedence.get(&TokenType::FUNCTION_CALL(0)).unwrap();

		//	and build an argument begin delimiter
		let mut lparen_arg_token = function_call_token.clone();
		lparen_arg_token.token_type = TokenType::LPAREN_ARG;
		lparen_arg_token.token_category = self.tokenizer.categories.get(&TokenType::LPAREN_ARG).unwrap().clone();
		//lparen_arg_token.token_precedence = *self.tokenizer.in_stack_precedence.get(&TokenType::LPAREN_ARG).unwrap();
		lparen_arg_token.token_value = "(".to_string();
		lparen_arg_token.line_number = function_call_token.line_number;
		lparen_arg_token.line_text = function_call_token.line_text.clone();


		//	get the location of the function call token in the infix_expression.  But we save it
		//  only if there are, in fact, any arguments to this call
		let loc = self.infix_expression.len();

		//	Add the function call to the infix expression
		self.infix_expression.push(function_call_token.clone());

		//	Add the Argument delimiter
		self.infix_expression.push(lparen_arg_token.clone());

		//	now, we're currently looking at the LPAREN that begins the function call's
		//  argument list, look ahead to see if, perhaps, there are no arguments.
		if self.look_ahead_test_token(TokenType::RPAREN){
			//	if see an RPAREN, we have to assume that this function call
			//  has no arguments.

			//	build an argument begin delimiter
			let mut rparen_arg_token = function_call_token.clone();
			rparen_arg_token.token_type = TokenType::RPAREN_ARG;
			rparen_arg_token.token_category = self.tokenizer.categories.get(&TokenType::RPAREN_ARG).unwrap().clone();
			//rparen_arg_token.token_precedence = *self.tokenizer.in_stack_precedence.get(&TokenType::RPAREN_ARG).unwrap();
			rparen_arg_token.token_value = ")".to_string();
			rparen_arg_token.line_number = function_call_token.line_number;
			rparen_arg_token.line_text = function_call_token.line_text.clone();

			//	Add the argument delimiter
			self.infix_expression.push(rparen_arg_token.clone());

			return None;
		}

		//	Create a comma counter for this function call
		self.comma_counters.push(0);

		//	and add the location of this call to the locations vector
		self.function_call_locations.push(loc);

		//	This is the LPAREN that starts the function call's argument list
		//self.infix_expression.push(self.token.clone());

		//	now we want to count parens so that we know when to backpatch the
		//	function call (we do it when this counter goes to 0).  We initialize
		//	it to 1 since we are looking the LPAREN that begins this function calls
		//	arguments
		self.function_call_paren_counters.push(1);

		self.parser_context.push(ParserContext::FunctionCallTerm);

		//	return a non-standard state
		Some(ParserState::Expression)
	}

	//	we get here because we saw a COMMA.  Increment the general
	//	comma counter (which counts commas across the entire expression)
	//
	//	There are two kinds of commas (really?):  list_separator and argument
	//	separator.  We convert the COMMA into one of these depending on
	//  the context
	
	//  For argument separators, We increment the general comma counter.  For
	//  list separator we just convert (we don't care how many there are)
	fn expression_list_item (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: expression_list_item \"{}\" context={:?}", self.token.token_value, self.parser_context)}
	
		let mut token = self.token.clone();

		let context = self.parser_context.last().unwrap();
		if *context == ParserContext::ArrayLiteral || *context == ParserContext::DictLiteral{
			//  Since we're in an ArrayLiteral context, convert the
			//  comma into a LIST_SEPARATOR
			token.token_type = TokenType::LIST_SEPARATOR;
			token.token_category = TokenCategory::ListSeparator;
		}else if *context == ParserContext::FunctionCallTerm || *context == ParserContext::FunctionCallStatement{
			self.statement_list_item_counter += 1;

			//	now is there a function call specific counters?
			let last = self.comma_counters.len() as i32 - 1;
	
			//  if so, then incremet it
			if last >= 0 {
				self.comma_counters[last as usize] += 1;
			}

			//	Since we're in a FunctionCallTerm or FunctionCallStatement context, convert the
			//  Comma to an ARG_SEPARATOR
			token.token_type = TokenType::ARG_SEPARATOR;
			token.token_category = TokenCategory::ArgSeparator;
		}else if *context == ParserContext::CollectionSource{
			//  just accept this contet
		}else{
			abend!(format!("from expression_list_item:  unknown parser context: {}", context));
		}

		self.infix_expression.push(token.clone());	

		//eprintln!("================== parser.expression_list_item: {} list={}",token, token_list_text(&self.infix_expression));

		None
	}


	fn literal_id(&mut self) -> Option<ParserState>{
		self.literal_id = self.token.clone();

		//	this is temporary since eventually we'll want the value to
		//	be a complete expression.  For now we're only interested in
		//	the first token in the list.
		self.literal_value.clear();
		None
	}

	fn literal_value(&mut self) -> Option<ParserState>{
		self.literal_value.push(self.token.clone());


		if self.current_state == ParserState::GlobalLiteralValue{
			self.model.add_global_literal(self.literal_id.clone(), self.literal_value.clone());
		}else if self.current_state == ParserState::LocalLiteralValue{
			self.model.add_literal_statement(self.literal_id.clone(), self.literal_value.clone());
		}else{
			panic!("from Parser.lieral_value: unknown parser state: {}", self.current_state);
		}
		None
	}

	fn verb (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: verb \"{}\" context:{:?} braces={}", self.token.token_value, self.parser_context, self.brace_counter);}
		self.infix_expression.clear();
		match self.token.token_type{
			TokenType::PRINT | TokenType::EPRINT | TokenType::PRINTLN | TokenType::EPRINTLN | TokenType::RETURN  =>  {
				self.parser_context.push(ParserContext::Simple);
				self.simple = self.token.clone();
			},

			TokenType::BREAK | TokenType::CONTINUE | TokenType::EXIT=> {
				self.parser_context.push(ParserContext::Simple);
				self.simple = self.token.clone();
				if self.look_ahead_test_token (TokenType::SEMI){
					return Some(ParserState::ExpressionTerm);
				}
			},
			TokenType::IF => self.parser_context.push(ParserContext::If),
			TokenType::WHILE => self.parser_context.push(ParserContext::While),
			TokenType::EVAL => {
				self.new_eval_brace_counter();
				self.parser_context.push(ParserContext::Eval);
			},
			TokenType::WHEN => if let ParserContext::Eval = self.parser_context.last().unwrap(){
				self.parser_context.push(ParserContext::When);
			}else{
				return Some(ParserState::Error(format!("Syntax Error:  from Parser.verb: 'WHEN' may only appear as an arm to EVALUATE")));
			},
  			_ => eprintln!("****** from verb:  unknown token {}", self.token.token_type),
		}

		None
	}

	fn keyword_otherwise (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: keyword_otherwise \"{}\" braces={}", self.token.token_value, self.brace_counter)}

		if let ParserContext::Eval = self.parser_context.last().unwrap(){
			self.parser_context.push(ParserContext::When);
		}else{
			return Some(ParserState::Error(format!("Syntax Error:  from Parser.verb: 'OTHERWISE' may only appear as an arm to EVALUATE")));
		}		


		self.model.add_otherwise_statement();

		if self.look_ahead_test_token(TokenType::LBRACE){
			//self.brace_counter += 1;
			self.increment_eval_brace_counter();
		}else{
			let token = self.next_token();
			return Some(ParserState::Error(format!("Syntax error from keyword_otherwise, line {}: Missing '{}' following OTHERWISE. Saw '{}' instead", line!(), '{', token.token_value)));
		}

		None
	}

	//	When we see an ELSE  we need to update the IF whose location in
	//	the model is at the top of the if_model_context stack.  For example:
	//
	//		if cond {				#1
	//			statement
	//			if cond {			#2
	//				statement
	//			}					#3
	//		}else{					#4
	//			if cond {			#5
	//				statement
	//			}else{				#6
	//				statement
	//			}
	//			statement
	//		}
	//
	//	When we add #1, we get it's location in the model and push
	//	it onto the if_model_context stack.
	//
	//	at #2, we add that IF to the model and push its location
	//	onto the if_model_context_stack
	//
	//	at #3, we detect that #2 does not have an ELSE so we pop
	//	the if_model_context because we won't be using it
	//
	//	at #4, we detect an ELSE so we go to the if_model_context
	//	grab the item at the top of the stack, use that to
	//	find #1 in the model, and set the has_else flag on that IF.
	//	After updating the IF we remove the item at the top of
	//	if_model_context_stack
	//
	//	if there was an ELSE at #3 we would use the top of the
	//	if_model_context to set the has_else on the #2 IF
	//
	//	at #5, we push a new value onto the if_model_context stack and
	//	because we removed the previous TOS of the if_model_context, this
	//	value will be the only one on the stack
	//
	//	at #6, we do the same thing as #4.
	
	fn keyword_else (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: keyword_else \"{}\"", self.token.token_value)}

		// there should be something on the if_model_context stack
		if self.if_model_context.is_empty(){
			abend!(format!("from keyword_else: Houston, we have a problem.  Expecting to see at least one value on if_model_context stack"));
		}
		// add the else statement (which also updates the has_else flag on the IF
		self.model.add_else_statement(self.if_model_context.pop().unwrap());

		if self.look_ahead_test_token(TokenType::LBRACE){
			self.parser_context.push(ParserContext::Else);
		}else{
			let token = self.next_token();
			return Some(ParserState::Error(format!("Syntax error from keyword_else, line {}: Missing '{}' following ELSE. Saw '{}' instead", line!(), '{', token.token_value)));
		}

		None
	}
	
	fn keyword_loop (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: keyword_loop \"{}\"", self.token.token_value)}
		self.model.add_loop_statement();
		None
	}

	//	This is called when the target factor is followed by '['. The assignment
	//	starget is indexed (e.g. a[1] = 10).  Set the context to IndexedTarget
	//	and change the token_type of the target to INDEXED_ID.
	fn indexed_target(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: indexed_target \"{}\"", self.token.token_value)}
	
		self.parser_context.push(ParserContext::IndexedTarget);

		if self.statement_factor.token_type == TokenType::ID{
			self.statement_factor.token_type = TokenType::INDEXED_ID;	
		}else if self.statement_factor.token_type == TokenType::QUALIFIED_ID{
			self.statement_factor.token_type = TokenType::INDEXED_QUALIFIED_ID
		}

		self.assignment_target_index_expression.clear();

		None
	}

	//	This is called when we see <factor>[ (for example "a[").  We know that an <index expression> will follow but
	//	what we need to know is if the statement is "a[x] = <expression>" or "a = b[<expression>]"
	//
	//  If the current state is Assignment then we know that it's the former and if the state is ExpressionTerm
	//  it's the latter.  Then when we see the "]", based on this knowledge, we know whether or not to expect
	//	some kind of assignment op or, possibly, more expression terms.  To transmit this to the future action
	//	"rbracket" needs to know what state to return to the statement machine.
	//
	//	Also, the generator needs to know that the brackets are LINDEX or RINDEX instead of LBracket or RBracket
	//
	//		When we save the token
	fn index_expression (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: index_expression.  CurrentState={} \"{}\"", self.current_state, self.token.token_value)}

		if self.current_state == ParserState::Assignment{
			//	if we  see a '[' after a <factor> the context is assignment
			self.parser_context.push(ParserContext::Assignment);
		}else if self.current_state == ParserState::ExpressionTerm{
			//	if we see a '[' in the middle of an expression then we assume
			//	we're fetching data from a collection
			self.parser_context.push(ParserContext::CollectionSource);
		}else{
			abend!(format!("from Action index_expression. Houston, we have a problem: Expecting the current state to be Assignment or ExpressionTerm but it was {}", self.current_state));
		}

		//	In any event, we convert the "[" as Lbracket to Lindex (i.e. the beginning
		//	of an index expression)
		self.infix_expression.push(self.bracket_to_index());
		None
	}

	//	We know definately that this is [1,2,3] so all we have to do here is set the context to ArrayLiteral
	//	however, we want to check for [] so that the  CPL program can say: create an empty array
	fn array_literal (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: array_literal \"{}\"", self.token.token_value)}

		if self.look_ahead_test_token(TokenType::RBRACKET){
			//	If we've seen x=[] then we do this hack
			self.next_token();
			let mut new_array_token = self.token.clone();
			new_array_token.token_type = TokenType::NEW_COLLECTION;
			new_array_token.token_category = TokenCategory::LBracket;
			self.infix_expression.push(new_array_token);
			return Some(ParserState::ExpressionTerm);
		}

		self.parser_context.push(ParserContext::ArrayLiteral);
		self.infix_expression.push(self.token.clone());
		None
	}

	fn dict_literal_begin (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: dict_literal_begin \"{}\"", self.token.token_value)}
		self.parser_context.push(ParserContext::DictLiteral);		
		let mut token = self.token.clone();
		token.token_type = TokenType::LDICT;
		token.token_category = TokenCategory::LDict;
		self.infix_expression.push(token.clone());
		None
	}
	fn dict_literal_kv_begin (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: dict_literal_kv_begin \"{}\"", self.token.token_value)}
		let mut token = self.token.clone();
		token.token_type = TokenType::LDICT_KV;
		token.token_category = TokenCategory::LDict_Kv;
		self.infix_expression.push(token.clone());
		None
	}
	fn dict_literal_end (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: dict_literal_end \"{}\"", self.token.token_value)}
		self.parser_context.pop();
		let mut token = self.token.clone();
		token.token_type = TokenType::RDICT;
		token.token_category = TokenCategory::RDict;
		self.infix_expression.push(token.clone());
		None
	}


	fn dict_literal_kv_end (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: dict_literal_kv_end \"{}\"", self.token.token_value)}

		//eprintln!(".......dict_literal_kv_end {}",self._parser_context_text());

		let mut token = self.token.clone();
		token.token_type = TokenType::RDICT_KV;
		token.token_category = TokenCategory::RDict_Kv;
		self.infix_expression.push(token.clone());
		None
	}

	//	This is called when we see an LParen token
	fn lparen(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: lparen \"{}\" context={:?}", self.token.token_value, self.parser_context)}		

		//	 increment the expression wide paren counter
		self.paren_counter += 1;
		self.infix_expression.push(self.token.clone());

		//	if there is an active function call specific paren counter, increment it
		let last = self.function_call_paren_counters.len() as i32 - 1;
		if last >= 0{
			self.function_call_paren_counters[last as usize] += 1;
		}
		None
	}

	//  RParen tokens are both precedence altering and function call terminators.
	//	if the parser context is FunctionCallStatement and the paren_counter goes to
	//	zero, then we know this is the end of the arguments to a function call statement
	fn rparen(&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: rparen \"{}\" context={:?}", self.token.token_value, self.parser_context)}

		//	first, decrement the global paren counter
		self.paren_counter -= 1;
		//	And, if the counter goes negative, we'e got paren mismatch
		if self.paren_counter < 0{
			abend!(format!("from Action rperen:  parenthesis mismatch!"));
		}

		//	if the current context is FunctionCallStatement then we're done
		if *self.parser_context.last().unwrap() == ParserContext::FunctionCallStatement{
			return None
		}

		//	if there is an active function call specific paren counter, decrement it
		//	and, if it should go to 0, do the backpatch and pop the counter
		let last = self.function_call_paren_counters.len() as i32 - 1;
		if last >= 0{
			self.function_call_paren_counters[last as usize] -= 1;

			//	if the function call specific paren counter goes to zero the
			//	we're done with this function call
			if self.function_call_paren_counters[last as usize] == 0{

				//  return to the outter context (e.g. Assignment)
				self.backpatch_expression_function_argument_count();
				self.parser_context.pop();

				//	We're done with this counter
				self.function_call_paren_counters.pop();

				//	Build and add the RPAREN_ARG token
				let mut rparen_arg_token = self.token.clone();
				rparen_arg_token.token_type = TokenType::RPAREN_ARG;
				rparen_arg_token.token_category = self.tokenizer.categories.get(&TokenType::RPAREN_ARG).unwrap().clone();
				//rparen_arg_token.token_precedence = *self.tokenizer.in_stack_precedence.get(&TokenType::RPAREN_ARG).unwrap();
				rparen_arg_token.token_value = ")".to_string();
				rparen_arg_token.line_number = self.token.line_number;
				rparen_arg_token.line_text = self.token.line_text.clone();

				//	Add the argument delimiter
				self.infix_expression.push(rparen_arg_token.clone());

				return None;
			}
		}

		self.infix_expression.push(self.token.clone());
		None
	}
	
	//	This is called whenever we see a ']'.  The return depends on the context:
	//
	//		Assignment -> Assignment (e.g. a[5] = <expression>) and "]" is RINDEX
	//		IndexedTarget -> Assignment save the RINDEX on the infix expression
	//		CollectionSource -> None (e.g. a  = b[5]) and "]" is RINDEX
	//		ArrayLiteral -> None (e.g a = [1,2,3];) and "]" stays RBRACKET
	//
	//	Reset the context to "Nothing"
	fn rbracket (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: rbracket \"{}\" context={:?}", self.token.token_value, self.parser_context)}

		//	Build a RINDEX
		let mut rindex = self.token.clone();
		rindex.token_type = TokenType::RINDEX;
		rindex.token_category = self.tokenizer.categories.get(&TokenType::RINDEX).unwrap().clone();

		match   self.parser_context.pop().unwrap(){
			ParserContext::Assignment => {
				//  If we see ']' in this context we know that the target was a collection element
				//	and we'll need to expect an assignment operator next
				self.infix_expression.push(rindex.clone());
				Some(ParserState::Assignment)  // overrides the transition
			},  
			ParserContext::CollectionSource => {
				//	If we see a ']' in this context we know we're done collecting index tokens
				self.infix_expression.push(rindex.clone());
				None			// the default is in the transition
			},
			ParserContext::ArrayLiteral =>{
				self.infix_expression.push(self.token.clone());
				None				// the default is in the transition
			},
			ParserContext::IndexedTarget => {
				//  Note, we don't include the ']' in the postfix becauase that
				//	will trigger a "fetch_indexed" instruction.  All we want here is
				//	whatever the number is.
				self.do_infix_to_postfix();
				self.assignment_target_index_expression = self.postfix_expression.clone();
				
				// get the assignment operator next
				Some(ParserState::Assignment)
			}
			_ => {
				eprintln!("From rbracket: Unexpected ParserContext value: {}", self.parser_context.last().unwrap());
				//  We might want to call this ane error
				None
			},
		}
	}

	fn lbrace (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: lbrace \"{}\" context={:?} braces={}", self.token.token_value, self.parser_context, self.brace_counter)}

		self.brace_counter += 1;

		//	When we see "{" we look at the context.
		//  and add these statements to the model
		match self.parser_context.last().unwrap(){
			ParserContext::If => {
				self.do_infix_to_postfix();
				self.if_model_context.push(self.model.add_if_statement(self.postfix_expression.clone()));
			},

			ParserContext::While => {
				self.do_infix_to_postfix();
				self.model.add_while_statement(self.postfix_expression.clone());
			},

			ParserContext::Foreach => {
				self.do_infix_to_postfix();
				for t in &self.postfix_expression{
					self.foreach_source.push(t.clone());
				}
				self.model.add_foreach_statement(self.foreach_target.clone(), self.foreach_source_type, self.foreach_source.clone());
			},

			ParserContext::Eval => {
				self.increment_eval_brace_counter();
				self.do_infix_to_postfix();
				self.eval_model_context.push(self.model.add_eval_statement(self.postfix_expression.clone()));
			},

			ParserContext::When => {
				self.increment_eval_brace_counter();
				self.do_infix_to_postfix();
				self.model.add_when_statement(self.postfix_expression.clone(), *self.eval_model_context.last().unwrap());
			},

			_=> abend!(format!("from Action lbrace:  I don't know what to do with this context: {}", self.parser_context.last().unwrap())),
		}
		self.infix_expression.clear();
		None
	}
	

	fn rbrace (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: rbrace \"{}\" context={:?} braces={}", self.token.token_value, self.parser_context, self.brace_counter)}

		match self.parser_context.last().unwrap(){
			ParserContext::Eval => {
				if self.decrement_eval_brace_counter() == 0{
					self.eval_model_context.pop();
					self.remove_eval_brace_counter();
					self.parser_context.pop();
				}
			},

			ParserContext::When => {
				self.parser_context.pop();
				self.decrement_eval_brace_counter();
			},

			ParserContext::If => {
				//	When we're in an IF context and we see the "}" we can see either
				//	and ELSE or something else as the next token.  If we don't see
				//	an ELSE, we don't care about this IF any more so we can remove
				//	its self_model_context from the stack (assuming there is one)

				//	see also the comments above "fn keyword_else"
				let if_ctx_len = self.if_model_context.len();
				if !self.look_ahead_test_token(TokenType::ELSE) && if_ctx_len > 0{
					if if_ctx_len > 0 {
						self.if_model_context.pop();
					}					
				}
				self.parser_context.pop();
			},

			ParserContext::Struct => {
				//	When we're processing a struct declaration, decrement the brace counter
				//	and if it goes to zero, go to state program.  No code is actually generated
				//	here.
				self.parser_context.pop();
				self.brace_counter -= 1;
				self.struct_ix.pop();
				if self.brace_counter <= 0{
					return None;
				}else{
					return Some(ParserState::StructBody);
				}
			},			
			_ => {
					if self.parser_context.len() > 1{
						self.parser_context.pop();
					}
			}, 
		}

		//	Decrement the brace counter, and if it goes to zero or less than 0)
		//	Then the function is done.  Add a block_end and goto state:Program
		//	where we might see other functions or structs.  Signal end of function
		//	in the call to add_block_end
		self.brace_counter -= 1;
		if self.brace_counter <= 0{
			self.model.add_block_end(true);
			return Some(ParserState::Program);
		}

		//	otherwise if the brace counter is still posititive (meaning
		//	the function is not complete) add the block_end signalling
		//	NOT the end of the function		
		self.model.add_block_end(false);
		None
	}

	//	When we recognize a FOREACH statement, initialize its variable
	fn foreach_init(&mut self) -> Option<ParserState>{
		self.foreach_source.clear();
		None
	}

	fn foreach_target (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: foreach_target \"{}\"", self.token.token_value)}
		if self.token.token_type != TokenType::ID{
			return Some(ParserState::Error(format!("Syntax error from foreach_target, line {}: Target of Foreach must be an ID", line!())));
		}
		self.foreach_target = self.token.clone();
		None		
	}

	//  the source can be an ID, an array literal or a dictionary literal.  If a literal, set the context
	//	appropriately, however literals aren't implemented yet in genstructs.
	//
	//	TODO:add support for literal arrays and dictionaries.
	fn foreach_source (&mut self) -> Option<ParserState>{
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Action: foreach_source \"{}\"", self.token.token_value)}

		//	we use the token type following the target to determine what kind
		//	of source will be used:  ID == scalar, LBRACKET = array, LBRACE = dictionary
		self.foreach_source_type = self.token.token_type;

		match self.token.token_type{
			TokenType::ID => {				// if ID add it to the source and we know enough to add it
				self.foreach_source.push(self.token.clone());
				self.model.add_foreach_statement(self.foreach_target.clone(), self.foreach_source_type, self.foreach_source.clone());

				//	assume we're going to see a "{" so increment the brace counter here. 
				self.brace_counter += 1;
			},		
			TokenType::LBRACKET | TokenType::LBRACE => abend!(format!("From foreach_source:  Foreach <target> <collection literal> not implemented yet.")),
			_=> return Some(ParserState::Error("From foreach_source:  Expecting an ID, '[', or '{'".to_string())),
		}

		None
	}

	/***********************************************************************************
	*******  UTILITY FUNCTIONS 
	***********************************************************************************/
	
	fn backpatch_expression_function_argument_count(&mut self){
		if self.cli.is_debug_bit(TRACE_PARSER_STATES) {eprintln!("    Sub-Action: backpatch_expression_function_argument_count")}

		//	So the way this works is we go to the "locations vector" and get the
		//	TOS.  this will be the location in the infix expression of the most recently
		//	recognized function call.  Then we get the comma counter at the top of that
		//	stack.  This will be number of arguments - 1.  We then update the
		//	the token type to include the argument counter.

		//	Okay, so here goes...
		let loc = self.function_call_locations.pop().unwrap();
		let count = self.comma_counters.pop().unwrap() + 1;
		self.infix_expression[loc].token_type = TokenType::FUNCTION_CALL(count as usize);
	}


	fn next_token(&mut self) -> Token{
		let mut rtn : Token ;
		loop{
			rtn = self.tokenizer.next_token();

			//  skip comments		
			if rtn.token_category != TokenCategory::Comment{
				break;
			}
		}

		if self.cli.is_debug_bit(TRACE_PARSER_NEXT_TOKEN) {
			eprintln!("parser/{}: <{}> \"{}\" (<{}>)"
				, "next_token"
				, rtn.token_type
				, rtn.token_value
				, rtn.line_text
			);
		}
		
	rtn
	}
	//	Read the next token and test for a match on a single token
	//	Returns true if there is return true, else return false.
	//
	//	after reading the next token, push it back onto the token
	//	list
	fn look_ahead_test_token(&mut self, token_type : TokenType) -> bool{
		//	Get the next non-comment token
		let local_token = self.next_token();

		//  but leave it behind because we don't actually want it now
		self.tokenizer.push_back();
		if local_token.token_type == token_type{
			return true;
		}else{
			return false;
		}
	}

	//	Read the next token and test for a match on a list of tokens
	fn look_ahead_test_multiple(&mut self, token_types : &Vec<TokenType>) -> bool{
		//	Get the next non-comment token
		let local_token = self.next_token();

		//  but leave it behind because we don't actually want it now
		self.tokenizer.push_back();

		if token_types.contains(&local_token.token_type){
			return true;
		}else{
			return false;
		}
	}
	
	//	Converts a bracket to an index delimiter
	fn bracket_to_index(&self) -> Token{
		let mut bracket = self.token.clone();
		
		if self.token.token_type == TokenType::LBRACKET{
			bracket.token_type = TokenType::LINDEX;
			bracket.token_category = TokenCategory::LIndex;
		}else if self.token.token_type == TokenType::RBRACKET{
			bracket.token_type = TokenType::RINDEX;
			bracket.token_category = TokenCategory::RIndex;
		}else{
			abend!(format!("from bracket_to_index.  Houston we have a problem.  Expected to see either ']' or '[' but saw: {}", self.token.token_type)); 
		}

		return bracket.clone();
	}


	//	Convert the current token from a binary op to unary op.
	fn binary_op_to_unary_op(&self) -> Token{
		let mut binary_op = self.token.clone();
		if self.token.token_type == TokenType::ADD{
			binary_op.token_type = TokenType::UPLUS;
		}else if self.token.token_type == TokenType::SUB{
			binary_op.token_type = TokenType::UMINUS;
		}else if self.token.token_type == TokenType::DAMNIT{
			// don't do anything for damnit because it's already a unary op
		}else{
			panic! ("from binary_op_to_unary_op.  Houston we have a problem.  Trying to convert an illegal binary op to unary {}", self.token.token_type);
		}
		
		binary_op.token_category = TokenCategory::UnaryOp;
		binary_op.clone()
	}

	
}
