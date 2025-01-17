#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::fs::File;
use std::io::prelude::*;
use std::env;
use std::io::BufReader;
use std::fmt;
use std::collections::HashSet;
use std::collections::HashMap;
use cli::*;
use macrolib::*;

pub const DEBUG_INDENT : &str = "==================== ";

//  The tokenizer reads an input file and produces an array of tokens.

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum TokenCategory{
	Misc,
	Verb,					//  A token indicating the beginning of a statement
	Else,					//  Need a standalone category for else since it's syntacticly different than a verb
	Foreach,				//  Another standalone category for foreach since it's also syntanctically different
	Loop,					//
	Otherwise,				// 
	Keyword,
	KeywordSimple,			//  keyword followed by an expression (e.g. print)
	StructDeclaration,		//  "struct"
	FunctionDeclaration,	//  "fn" or "entry"
	Factor,					// Id, Number, Boolean, String
	Literal,				// Keyword Litreal
	FunctionCall,
	IndexedId,
	IndexedQualifiedId,		// may not need this
	Comment,
	AssignmentOp,
	BinaryOp,
	UnaryOp,
	RelOp,					// && and ||
	IncDec,
	LBrace,
	RBrace,
	LBracket,
	RBracket,
	LIndex,
	RIndex,
	LParen,
	RParen,
	LParen_Arg,
	RParen_Arg,
	Semi,
	Comma,
	ListSeparator,
	ArgSeparator,
	Unknown,
	BlockMarker,

	LDict,
	RDict,
	LDict_Kv,
	RDict_Kv,

	IndexExpression,
	ArgumentExpression,
	Internal,
}

impl fmt::Display for TokenCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
			TokenCategory::Misc					=> write!(f, "Misc"),
			TokenCategory::Verb					=> write!(f, "Verb"),
			TokenCategory::Else					=> write!(f, "Else"),
			TokenCategory::Foreach				=> write!(f, "Foreach"),
			TokenCategory::Loop					=> write!(f, "Loop"),
			TokenCategory::Otherwise			=> write!(f, "Otherwise"),
			TokenCategory::Keyword				=> write!(f, "Keyword"),
			TokenCategory::KeywordSimple		=> write!(f, "KeywordSimple"),
			TokenCategory::FunctionDeclaration	=> write!(f, "FunctionDeclaration"),
			TokenCategory::StructDeclaration	=> write!(f, "StructDeclaration"),
			TokenCategory::Factor				=> write!(f, "Factor"),
			TokenCategory::Literal				=> write!(f, "Literal"),
			TokenCategory::FunctionCall			=> write!(f, "FunctionCall"),
			TokenCategory::IndexedId			=> write!(f, "IndexedId"),
			TokenCategory::IndexedQualifiedId	=> write!(f, "IndexedQualifiedId"),
			TokenCategory::Comment				=> write!(f, "Comment"),
			TokenCategory::AssignmentOp			=> write!(f, "AssignmentOp"),
			TokenCategory::BinaryOp				=> write!(f, "BinaryOp"),
			TokenCategory::UnaryOp				=> write!(f, "UnaryOp"),
			TokenCategory::RelOp				=> write!(f, "RelOp"),
			TokenCategory::IncDec				=> write!(f, "IncDec"),
			TokenCategory::LBrace				=> write!(f, "LBrace"),
			TokenCategory::RBrace				=> write!(f, "RBrace"),
			TokenCategory::LBracket				=> write!(f, "LBracket"),
			TokenCategory::RBracket				=> write!(f, "RBracket"),
			TokenCategory::LIndex				=> write!(f, "LIndex"),
			TokenCategory::RIndex				=> write!(f, "RIndex"),
			TokenCategory::LParen				=> write!(f, "LParen"),
			TokenCategory::RParen				=> write!(f, "RParen"),

			TokenCategory::LParen_Arg			=> write!(f, "LParen_Arg"),
			TokenCategory::RParen_Arg			=> write!(f, "RParen_Arg"),

			TokenCategory::LDict				=> write!(f, "LDict"),
			TokenCategory::RDict				=> write!(f, "RDict"),
			TokenCategory::LDict_Kv				=> write!(f, "LDict_Kv"),
			TokenCategory::RDict_Kv				=> write!(f, "RDict_Kv"),

			TokenCategory::Semi					=> write!(f, "Semi"),

			TokenCategory::Comma				=> write!(f, "Comma"),
			TokenCategory::ListSeparator		=> write!(f, "ListSeparator"),
			TokenCategory::ArgSeparator			=> write!(f, "ArgSeparator"),

			TokenCategory::IndexExpression		=> write!(f, "IndexExpression"),
			TokenCategory::ArgumentExpression	=> write!(f, "ArgumentExpression"),

			TokenCategory::Unknown				=> write!(f, "Unknown"),
			TokenCategory::Internal				=> write!(f, "Internal"),
			TokenCategory::BlockMarker			=> write!(f, "BlockMarker"),
		}	   
	}
}

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum TokenType{
 	/* keywords that have parenthetic components */
	NONE,						// Used for initialization only
	UNKNOWN,
	ID,							// unquoted string in symbol table
	QUALIFIED_ID,				// E.G.  A:B:C
	FUNCTION_CALL(usize),		// in an expression context the usize is the argument count
	STRING,						// quoted with either ' or "
	INTEGER,
	FLOAT,
	BOOL,						// "true" or "false"
	EOF,						// end of input file
	KEYWORD_OR_ID,
	ERROR,

	LINE_COMMENT,
	BLOCK_COMMENT,

	ELSE, 
	DO,

	IF,
	ELSEIF,
	UNTIL,
	FOR,
	FOREACH,
	WHILE,
	LOOP,

	EVAL,
	WHEN,
	OTHERWISE,
	
	  /* keywords that work by themselves */
	
	FN,
	STRUCT,
	NEW,			// used to instantiate a struct,
	ASSIGNMENT,
	ENTRY,
	INCLUDE,
	BREAK,
	CONTINUE,
	RETURN,
	IN,
	TRUE,
	FALSE,

	PRINT,			//  println to stdout
	EPRINT,			//  println to stderr
	PRINTLN,		//  println to stdout
	EPRINTLN,		//  println to stderr

	EXIT,			//  exit a program back to OS

	EQ,
	GE,
	LE,
	NE,
	GT,
	LT,

	LOR,
	LAND,

	BITWISE_OR,
	BITWISE_AND,

	PP,
	MM,

	UMINUS,				// negation
	UPLUS,				// make positive
	LENGTH_OF,			// returns the length of something

	//	Assigment operators
	ASG_ADD_EQ,			// '+='
	ASG_SUB_EQ,			// '-='
	ASG_DIV_EQ,			// '/='
	ASG_MUL_EQ,			// '*='
	ASG_MOD_EQ,			// '%='
	ASG_APPEND_EQ,		// '.='
	ASG_OR_EQ,			// '|='
	ASG_AND_EQ,			// '&-'
	ASG_EQ,				// '='

	ADD,
	SUB,
	MUL,
	DIV,
	MOD,
	CONCAT,

	LPAREN,
	RPAREN,
	LBRACE,
	RBRACE,
	
	DAMNIT,

	SEMI,
	COMMA,

	LITERAL,

	//  Synthetic Tokens.  These are created by transforming a "normal" token
	//	into a special token used in specific contexts
	LBRACKET,
	RBRACKET,
	LINDEX,		// these are used  instead of brackets to surround index expressions
	RINDEX,

	LDICT,		// "{" as the start of  dictionary literal
	RDICT,		// "}" as the end of a dictionary literal
	LDICT_KV,	// "{" as the beginning of a a key-value pair
	RDICT_KV,	// "}" as the end of a key-value pair

	LPAREN_ARG,
	RPAREN_ARG,
	INDEXED_ID,
	INDEXED_QUALIFIED_ID,
	LIST_SEPARATOR,		// A comma within the context of a list
	ARG_SEPARATOR,		// A comma within the context of function call
	IDADDR,				// When '&' is recognized as unary operator, ID -> IDADDR

	INDEX_EXPRESSION,
	ARGUMENT_EXPRESSION,
	NEW_COLLECTION,			// when x=[] is encountered this is the expression token
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
			TokenType::NONE  => write!(f, "NONE"),
			TokenType::UNKNOWN  => write!(f, "UNKNOWN"),
			TokenType::KEYWORD_OR_ID  => write!(f, "KEYWORD_OR_ID"),
			TokenType::EOF  => write!(f, "EOF"),
			TokenType::ERROR  => write!(f, "ERROR"),
			TokenType::ID  => write!(f, "ID"),
			TokenType::QUALIFIED_ID  => write!(f, "QUALIFIED_ID"),
			TokenType::FUNCTION_CALL(_) => write!(f, "FUNCTION_CALL"),
			TokenType::STRING  => write!(f, "STRING"), 
			TokenType::INTEGER  => write!(f, "INTEGER"), 
			TokenType::FLOAT  => write!(f, "FLOAT"), 
			TokenType::BOOL  => write!(f, "BOOL"),
			TokenType::LINE_COMMENT  => write!(f, "LINE_COMMENT"),
			TokenType::BLOCK_COMMENT  => write!(f, "BLOCK_COMMENT"),
			TokenType::ELSE  => write!(f, "ELSE"), 
			TokenType::DO  => write!(f, "DO"),
			TokenType::IF  => write!(f, "IF"),
			TokenType::ELSEIF  => write!(f, "ELSEIF"),
			TokenType::UNTIL  => write!(f, "UNTIL"),
			TokenType::FOR  => write!(f, "FOR"),
			TokenType::FOREACH  => write!(f, "FOREACH"),
			TokenType::WHILE  => write!(f, "WHILE"),
			TokenType::LOOP => write!(f, "LOOP"),

			TokenType::EVAL  => write!(f, "EVAL"),
			TokenType::WHEN  => write!(f, "WHEN"),
			TokenType::OTHERWISE  => write!(f, "OTHERWISE"),

			TokenType::FN  => write!(f, "FN"),
			TokenType::STRUCT  => write!(f, "STRUCT"),
			TokenType::NEW  => write!(f, "NEW"),
			TokenType::ASSIGNMENT => write!(f, "ASSIGNMENT"),

			TokenType::PRINT => write!(f, "PRINT"),
			TokenType::EPRINT => write!(f, "EPRINT"),
			TokenType::PRINTLN => write!(f, "PRINT"),
			TokenType::EPRINTLN => write!(f, "EPRINT"),
			TokenType::EXIT => write!(f, "EXIT"),

			TokenType::ENTRY  => write!(f, "ENTRY"),
			TokenType::INCLUDE  => write!(f, "INCLUDE"),
			TokenType::BREAK  => write!(f, "BREAK"),
			TokenType::CONTINUE  => write!(f, "CONTINUE"),
			TokenType::RETURN  => write!(f, "RETURN"),
			TokenType::IN  => write!(f, "IN"),
			TokenType::TRUE  => write!(f, "TRUE"),
			TokenType::FALSE  => write!(f, "FALSE"),

			TokenType::EQ  => write!(f, "EQ"),
			TokenType::GE  => write!(f, "GE"),
			TokenType::LE  => write!(f, "LE"),
			TokenType::NE  => write!(f, "NE"),
			TokenType::GT  => write!(f, "GT"),
			TokenType::LT  => write!(f, "LT"),
	   
			TokenType::LOR  => write!(f, "LOR"),
			TokenType::LAND  => write!(f, "LAND"),

			TokenType::BITWISE_OR  => write!(f, "BITWISE_OR"),
			TokenType::BITWISE_AND  => write!(f, "BITWISE_AND"),
	   
			TokenType::PP  => write!(f, "PP"),
			TokenType::MM  => write!(f, "MM"),

			TokenType::UMINUS  => write!(f, "UMINUS"),
			TokenType::UPLUS  => write!(f, "UPLUS"),
			TokenType::LENGTH_OF  => write!(f, "LENGTH_OF"),
		
			TokenType::ASG_ADD_EQ  => write!(f, "ASG_ADD_EQ"),
			TokenType::ASG_SUB_EQ  => write!(f, "ASG_SUB_EQ"),
			TokenType::ASG_DIV_EQ  => write!(f, "ASG_DIV_EQ"),
			TokenType::ASG_MUL_EQ  => write!(f, "ASG_MUL_EQ"),
			TokenType::ASG_MOD_EQ  => write!(f, "ASG_MOD_EQ"),
			TokenType::ASG_APPEND_EQ  => write!(f, "ASG_APPEND_EQ"),
			TokenType::ASG_OR_EQ  => write!(f, "ASG_OR_EQ"),
			TokenType::ASG_AND_EQ  => write!(f, "ASG_AND_EQ"),
			TokenType::ASG_EQ  => write!(f, "ASG_EQ"),

			TokenType::ADD  => write!(f, "ADD"),
			TokenType::SUB  => write!(f, "SUB"),
			TokenType::DIV  => write!(f, "DIV"),
			TokenType::MUL  => write!(f, "MUL"),
			TokenType::MOD  => write!(f, "MOD"),
			TokenType::CONCAT  => write!(f, "CONCAT"),

			TokenType::LPAREN  => write!(f, "LPAREN"),
			TokenType::RPAREN  => write!(f, "RPAREN"),

			TokenType::LBRACE  => write!(f, "LBRACE"),
			TokenType::RBRACE  => write!(f, "RBRACE"),

			TokenType::DAMNIT  => write!(f, "DAMNIT"),

			TokenType::SEMI  => write!(f, "SEMI"),
			TokenType::COMMA  => write!(f, "COMMA"),

			TokenType::LITERAL  => write!(f, "LITERAL"),	   

			TokenType::LBRACKET  => write!(f, "LBRACKET"),
			TokenType::RBRACKET  => write!(f, "RBACKET"),
			TokenType::LINDEX  => write!(f, "LINDEX"),
			TokenType::RINDEX  => write!(f, "RINDEX"),
			TokenType::LPAREN_ARG  => write!(f, "LPAREN_ARG"),
			TokenType::RPAREN_ARG  => write!(f, "RPAREN_ARG"),

			TokenType::LDICT  => write!(f, "LDICT"),
			TokenType::RDICT  => write!(f, "RDICT"),
			TokenType::LDICT_KV  => write!(f, "LDICT_KV"),
			TokenType::RDICT_KV  => write!(f, "RDICT_KV"),

			TokenType::INDEXED_ID  => write!(f, "INDEXED_ID"),
			TokenType::INDEXED_QUALIFIED_ID  => write!(f, "INDEXED_QUALIFIED_ID"),
			TokenType::LIST_SEPARATOR  => write!(f, "LIST_SEPARATOR"),
			TokenType::ARG_SEPARATOR  => write!(f, "ARG_SEPARATOR"),	
			TokenType::IDADDR  => write!(f, "&ID"),

			TokenType::INDEX_EXPRESSION => write!(f, "INDEX_EXPRESSION"),
			TokenType::ARGUMENT_EXPRESSION => write!(f, "ARGUMENT_EXPRESSION"),
			TokenType::NEW_COLLECTION => write!(f, "NEW_COLLECTION"),
		}
    }
}


	//#[derive(PartialOrd, PartialEq, Eq, Copy, Clone)]
#[derive(Debug, PartialEq, Eq, Hash)]
enum TokenizerStates{
	BASE,
	BOT,
	OPERATOR,
	ERR,
	SINGLE_Q,
	DOUBLE_Q,
	NUMBER,
	REAL_NUMBER,
	KEYWORD_OR_ID,
	LINE_COMMENT,
	BLOCK_COMMENT,
	EOF,
	EOT,				// used to tell the state machine that we're done collecting the current token
}

impl fmt::Display for TokenizerStates {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match self {
           TokenizerStates::BASE => write!(f, "TokenizerStates::BASE"),
           TokenizerStates::BOT => write!(f, "TokenizerStates::BOT"),
           TokenizerStates::OPERATOR => write!(f, "TokenizerStates::OPERATOR"),
           TokenizerStates::ERR => write!(f, "TokenizerStates::ERR"),
           TokenizerStates::SINGLE_Q => write!(f, "TokenizerStates::SINGLE_Q"),
           TokenizerStates::DOUBLE_Q => write!(f, "TokenizerStates::DOUBLE_Q"),
           TokenizerStates::NUMBER => write!(f, "TokenizerStates::NUMBER"),
           TokenizerStates::REAL_NUMBER => write!(f, "TokenizerStates::REAL_NUMBER"),
           TokenizerStates::KEYWORD_OR_ID => write!(f, "TokenizerStates::KEYWORD_OR_ID"),
           TokenizerStates::LINE_COMMENT => write!(f, "TokenizerStates::LINE_COMMENT"),
           TokenizerStates::BLOCK_COMMENT => write!(f, "TokenizerStates::BLOCK_COMMENT"),
           TokenizerStates::EOF => write!(f, "TokenizerStates::EOF"),
           TokenizerStates::EOT => write!(f, "TokenizerStates::EOT"),
       }
    }
}

#[derive(Debug,Clone,PartialEq, Eq)]
pub struct Token{
	pub token_type : TokenType,
	pub token_value : String,
	pub line_number : i32,
	pub line_text : String,
	pub token_category : TokenCategory,
}

impl Token{
	pub fn new () -> Token{
		Token{
			token_type:TokenType::NONE,
			token_value:String::new(),
			line_number:0,
			line_text:String::new(),
			token_category : TokenCategory::Unknown,
		}
	}

	pub fn new2(ttype:TokenType, value:String, line_number:i32, text:String, cat:TokenCategory) -> Token{
		Token{
			token_type:ttype,
			token_value:value,
			line_number:line_number,
			line_text:text,	
			token_category:cat,
		}
	}

	pub fn make_string(val : &str) -> Token{
		Token{
			token_type:TokenType::STRING,
			token_value:val.to_string(),
			line_number:99999,
			line_text:"make_string".to_string(),	
			token_category:TokenCategory::Internal
		}
	}
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} {} {} [\"{}\"]", self.token_type, self.token_category, self.token_value, self.line_text)
	}
}

pub fn token_list_text(tokens : &Vec<Token>) -> String{
	if tokens.is_empty(){
		return "<empty>".to_string();
	}
	let mut display = String::new();
	let mut i=0;
	for t in tokens{
		if i > 0{
			display.push_str(" ");
		}

		if t.token_type == TokenType::INTEGER || t.token_type == TokenType::FLOAT || t.token_type == TokenType::STRING ||t.token_type==TokenType::BOOL{
			display.push_str(&format!("\"{}\"",t.token_value));
		}else{
			display.push_str(&format!("{}<{}>",t.token_value, t.token_type));
		}
		i+=1;
	}
	display
}

pub struct Tokenizer<'a>{
    tokenizer_state : TokenizerStates,
    reader : Vec<BufReader<File>>,
	include_pending : bool,
	line : String,
    line_index : usize,
	line_number : i32,
	pub token : Token,
	last_char : Vec<char>,
	error_text : String,
	cli : & 'a CLI<'a>,
	raw_tokens : Vec<Token>,
	raw_index : usize,

	pub categories : HashMap<TokenType,TokenCategory>,
}

impl<'a> Tokenizer<'_>{
    pub fn new (pcli : & 'a CLI<'a>) -> Tokenizer<'a>{

      	let mut izer = Tokenizer{
            tokenizer_state : TokenizerStates::BASE,
			reader : Vec::new(),
			include_pending : false,
			line : String::new(),
            line_index : 0,
			line_number : 0,
			token : Token::new(),
			last_char : Vec::new(),
			error_text : String::new(),
			cli : pcli,
			raw_tokens : Vec::new(),
			raw_index : 0,

			categories : HashMap::from (
				[
					(TokenType::FN,TokenCategory::FunctionDeclaration),
					(TokenType::STRUCT,TokenCategory::StructDeclaration),
					(TokenType::NEW,TokenCategory::Keyword),
					(TokenType::ENTRY,TokenCategory::FunctionDeclaration),

					(TokenType::DO,TokenCategory::Verb),
					(TokenType::IF,TokenCategory::Verb),
					(TokenType::FOR,TokenCategory::Verb),
					(TokenType::WHILE,TokenCategory::Verb),

					(TokenType::EVAL,TokenCategory::Verb),
					(TokenType::WHEN,TokenCategory::Verb),

					(TokenType::PRINT, TokenCategory::Verb),
					(TokenType::EPRINT, TokenCategory::Verb),
					(TokenType::PRINTLN, TokenCategory::Verb),
					(TokenType::EPRINTLN, TokenCategory::Verb),

					(TokenType::EXIT, TokenCategory::Verb),

					(TokenType::RETURN,TokenCategory::Verb),
					(TokenType::INCLUDE,TokenCategory::Verb),
					(TokenType::LITERAL,TokenCategory::Literal),

					//	Need to recognize special case verbs
					(TokenType::ELSE,TokenCategory::Else), 
					(TokenType::FOREACH,TokenCategory::Foreach),
					(TokenType::LOOP,TokenCategory::Loop),
					(TokenType::OTHERWISE,TokenCategory::Otherwise),

					(TokenType::BREAK,TokenCategory::Verb),
					(TokenType::CONTINUE,TokenCategory::Verb),

					(TokenType::ID,TokenCategory::Factor),
					(TokenType::QUALIFIED_ID,TokenCategory::Factor),
					(TokenType::STRING,TokenCategory::Factor),			// quoted with either ' or "
					(TokenType::INTEGER,TokenCategory::Factor),
					(TokenType::FLOAT,TokenCategory::Factor),
					(TokenType::BOOL,TokenCategory::Factor),

					(TokenType::FUNCTION_CALL(0),TokenCategory::FunctionCall),
					(TokenType::INDEXED_ID,TokenCategory::IndexedId),
					(TokenType::INDEXED_QUALIFIED_ID,TokenCategory::IndexedId),
				
					(TokenType::ASSIGNMENT,TokenCategory::Keyword),
					
					(TokenType::ELSEIF,TokenCategory::Keyword),
					(TokenType::UNTIL,TokenCategory::Keyword),

					(TokenType::IN,TokenCategory::Keyword),
					(TokenType::TRUE,TokenCategory::Factor),
					(TokenType::FALSE,TokenCategory::Factor),

					(TokenType::EQ,TokenCategory::BinaryOp),
					(TokenType::GE,TokenCategory::BinaryOp),
					(TokenType::LE,TokenCategory::BinaryOp),
					(TokenType::NE,TokenCategory::BinaryOp),
					(TokenType::GT,TokenCategory::BinaryOp),
					(TokenType::LT,TokenCategory::BinaryOp),
				
					(TokenType::LOR,TokenCategory::RelOp),
					(TokenType::LAND,TokenCategory::RelOp),
				
					(TokenType::BITWISE_OR,TokenCategory::BinaryOp),
					(TokenType::BITWISE_AND,TokenCategory::BinaryOp),
				
					(TokenType::PP,TokenCategory::IncDec),
					(TokenType::MM,TokenCategory::IncDec),

					(TokenType::UMINUS,TokenCategory::UnaryOp),
					(TokenType::UPLUS,TokenCategory::UnaryOp),
					(TokenType::LENGTH_OF,TokenCategory::UnaryOp),
					(TokenType::DAMNIT,TokenCategory::UnaryOp),

					(TokenType::ADD,TokenCategory::BinaryOp),
					(TokenType::SUB,TokenCategory::BinaryOp),
				
					(TokenType::MUL,TokenCategory::BinaryOp),
					(TokenType::DIV,TokenCategory::BinaryOp),
					(TokenType::MOD,TokenCategory::BinaryOp),
					(TokenType::CONCAT,TokenCategory::BinaryOp),

					(TokenType::LPAREN,TokenCategory::LParen),
					(TokenType::RPAREN,TokenCategory::RParen),

					(TokenType::LPAREN_ARG,TokenCategory::LParen_Arg),
					(TokenType::RPAREN_ARG,TokenCategory::RParen_Arg),

					(TokenType::LDICT,  TokenCategory::LDict),
					(TokenType::RDICT,  TokenCategory::RDict),
					(TokenType::LDICT_KV,  TokenCategory::LDict_Kv),
					(TokenType::RDICT_KV,  TokenCategory::RDict_Kv),
		
					(TokenType::LBRACE,TokenCategory::LBrace),
					(TokenType::RBRACE,TokenCategory::RBrace),
					(TokenType::LBRACKET,TokenCategory::LBracket),
					(TokenType::RBRACKET,TokenCategory::RBracket),
					(TokenType::LINDEX,TokenCategory::LIndex),
					(TokenType::RINDEX,TokenCategory::RIndex),
				
					(TokenType::SEMI,TokenCategory::Semi),

					(TokenType::COMMA,TokenCategory::Comma),
					(TokenType::LIST_SEPARATOR,TokenCategory::ListSeparator),
					(TokenType::ARG_SEPARATOR,TokenCategory::ArgSeparator),

					(TokenType::ASG_ADD_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_SUB_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_DIV_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_MUL_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_MOD_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_APPEND_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_OR_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_AND_EQ,TokenCategory::AssignmentOp),
					(TokenType::ASG_EQ,TokenCategory::AssignmentOp),

					(TokenType::NONE,TokenCategory::Misc),			// Used for initialization only
					(TokenType::UNKNOWN,TokenCategory::Misc),
			
					(TokenType::EOF,TokenCategory::Misc),
					(TokenType::KEYWORD_OR_ID,TokenCategory::Misc),
					(TokenType::ERROR,TokenCategory::Misc),
			
					(TokenType::LINE_COMMENT,TokenCategory::Comment),
					(TokenType::BLOCK_COMMENT,TokenCategory::Comment),
			
					(TokenType::INDEX_EXPRESSION,TokenCategory::IndexExpression),
					(TokenType::ARGUMENT_EXPRESSION,TokenCategory::ArgumentExpression),	
					(TokenType::NEW_COLLECTION,TokenCategory::Unknown),	
				]
			),
		};

		let file = File::open(pcli.source());
		match file{
			Err	(m) => {println!("       Can't open File : {}",pcli.source()); std::process::exit(1);},
			Ok(f) => izer.reader.push(BufReader::new(f)),
		}


		//	Build the raw_tokens list (so that next token and push_back work)
		izer.tokenize();
		izer
    }


	pub fn get_category(&self, token_type:TokenType) -> TokenCategory{
		match self.categories.get(&token_type){
			None => {}
			Some(x) => return *x,
		}
		println!("get_category:  can't find token {}",token_type);
		TokenCategory::Unknown
	}
	fn is_qualified_id (&self) -> bool{
		if self.token.token_value.find(':') != None{
			return true;
		}else{
			return false;
		}
	}

	//  The token looks like a keyword.  Try to find it in our list of known
	//	keywords and if it's not would then assume the toke is an ID
	fn determine_keyword(&mut self) -> TokenType{
		self.token.line_number = self.line_number;
		match self.token.token_value.as_str(){
			"else"			=> TokenType::ELSE, 
			"do"			=> TokenType::DO,
			"if"			=> TokenType::IF,
			"elseif"		=> TokenType::ELSEIF,
			"until"			=> TokenType::UNTIL,
			"for"			=> TokenType::FOR,
			"foreach"		=> TokenType::FOREACH,
			"while"			=> TokenType::WHILE,
			"loop"			=> TokenType::LOOP,

			"eval"			=> TokenType::EVAL,
			"when"			=> TokenType::WHEN,
			"otherwise"		=> TokenType::OTHERWISE,

			"fn"			=> TokenType::FN,
			"struct"		=> TokenType::STRUCT,
			"new"			=> TokenType::NEW,
			"entry"			=> TokenType::ENTRY,
			"include"		=> TokenType::INCLUDE,
			"break"			=> TokenType::BREAK,
			"continue"		=> TokenType::CONTINUE,
			"return"		=> TokenType::RETURN,
			"in"			=> TokenType::IN,
			"true"			=> TokenType::BOOL,
			"false"			=> TokenType::BOOL,
			"print"			=> TokenType::PRINT,
			"eprint"		=> TokenType::EPRINT,
			"println"		=> TokenType::PRINTLN,
			"eprintln"		=> TokenType::EPRINTLN,
			"exit"			=> TokenType::EXIT,

			"literal"		=> TokenType::LITERAL,
			"lit"			=> TokenType::LITERAL,
			
			//	Nope, it's an ID.  It might be a qualified ID (e.g. foo:bar)
			_ 				=> 	if self.is_qualified_id(){
									TokenType::QUALIFIED_ID
								}else{
									TokenType::ID
								}

		}
	}



	//  Fetch the next character from the reader buffer, reading the next
	//	line from the file if that buffer is exhausted.  Returns a character or None
	//	at end of file
    fn next_char(&mut self) -> Option<char>{
		//  if there is anything in the last_char stack use it instead of
		//	any new chracters from the file
		if !self.last_char.is_empty(){
			return Some(self.last_char.pop().unwrap());
		}

        if self.line.len() == 0 || self.line_index >= self.line.len(){
            self.line.clear();
			let reader_number = self.reader.len() - 1;
            if self.reader.get_mut(reader_number).unwrap().read_line(&mut self.line).unwrap() == 0{
                return None
            }
            self.line_index = 0;
			self.line_number += 1;
        }
        let c = self.line.as_bytes() [self.line_index] as char;
        self.line_index += 1;
        Some(c)
    }

	//	Deal with an Op that has multiple characters.  Returns true if the look ahead
	//	character matches the one passed, else returns false.  A side effect is that
	//	the tokenizer state is set to EOT since, for ops, it is the end of the token
	//	If the look_ahead character matches the "lookfor" then it is consumed (i.e. it
	//	isn't pushed onto the last_char stack )
	fn op_look_ahead(&mut self, input : char, lookfor : char) -> bool{
		if self.cli.is_debug_bit(TRACE_TOKENIZER){print!("op_look_ahead:  ......'{}' is looking for '{}' ", input, lookfor);}

		//	get the look ahead
		//self.tokenizer_state = TokenizerStates::EOT;
		let look_ahead_char = self.next_char();

		if look_ahead_char == None{
			return false;
		}

		if self.cli.is_debug_bit(TRACE_TOKENIZER){println!("found '{}'", look_ahead_char.unwrap());}
		if look_ahead_char.unwrap() == lookfor{
			return true;
		}

		self.last_char.push(look_ahead_char.unwrap());
		//println!("tokenizer op_look_ahead:  ......killing white space");
		//self.last_no_white_space(look_ahead_char.unwrap());
		false
	}

	//  handle EOF processing.  Returns an EOF token
	fn EOF (&self) -> Token{
		let mut rtn = Token::new();
		rtn.token_type = TokenType::EOF;
		rtn.line_number = self.line_number;
		if self.cli.is_debug_bit(TRACE_TOKENIZER) {println!("Tokenizer.EOF at line {}",rtn.line_number);}
		return rtn;
	}
	
	//	Read all of the tokens from the file and save them
	//	in a vector which, when the file is done, is returned
	pub fn tokenize(&mut self){
		let mut token = self.machine();
		while token.token_type != TokenType::EOF{
			//println!(".....{} \"{}\"", token.token_type, token.token_value);
			self.raw_tokens.push(token);
			token = self.machine();
		}
		//  End the list with the EOF
		self.raw_tokens.push(token);
	}


	//	retrieves the next token from the raw_tokens list and increments the index
	pub fn next_token(&mut self) -> Token{
		let rtn = self.raw_tokens[self.raw_index].clone();
		self.raw_index += 1;
		rtn
	}

	//	Returns the token "back_counter" tokens back.  Crashes if
	//	there aren't enuf tokens in the list to satisfy the request.
	//	NOTE:  the function also crashes if the back counter is < 1 (i.e.
	//	we only want to look backwords BEFORE the current token)
	pub fn prev_token(&self, back_counter : i32) -> Token{
		if back_counter < 1{
			abend!(format!("Houston, we have a problem! A previous token was requested with a back counter < 1"));
		}
		if self.raw_index as i32 - back_counter < 0 {
			abend!(format!("Houston, we have a problem! A previous token was requested but there isn't one -- the index - modifier is < 0"));
		}
		self.raw_tokens[self.raw_index - back_counter as usize].clone()
	}

	//	assuming we're somewhere in the middle of the raw_tokens list,
	//	reduce the index by 1, thereby putting pushing the current token back
	//	on the list
	pub fn push_back(&mut self){
		if self.raw_index > 0{
			self.raw_index -= 1;
		}
	}
	
	//	Tokenizer state machine.  Actual analysis is done in the corresponding state
	//	functions.  Callers will use the token variable in this struct rather than
	//	getting it returned to them.
    fn machine (&mut self) -> Token{
		//  Loop until ERR, EOT or EOF
		loop{
			match self.tokenizer_state{
				TokenizerStates::ERR 			=> {
					abend!(format!("{}", self.error_text));
				},
				TokenizerStates::EOT			=> {
					self.tokenizer_state = TokenizerStates::BASE;
					self.token.line_number = self.line_number;
					self.token.line_text = self.line.trim().to_string();

					if self.include_pending {
						let file = File::open(&self.token.token_value);
						match file{
							Err	(m) => {println!("       Can't open INCLUDE File : {}",self.token.token_value); std::process::exit(1);},
							Ok(f) => self.reader.push(BufReader::new(f)),
						}

						self.include_pending = false;
						self.tokenizer_state = TokenizerStates::BASE;
						continue;
					}

					if self.token.token_type == TokenType::KEYWORD_OR_ID{
						self.token.token_type = self.determine_keyword();

						if self.token.token_type == TokenType::INCLUDE{
							self.include_pending = true;
							continue;
						}
					}
			
					self.token.token_category = *self.categories.get(&self.token.token_type).unwrap();
			
					if self.cli.is_debug_bit(TRACE_TOKENIZER) {println! ("Tokenizer.next_token/State::EOT '{}'",self.token.token_value);}
					return self.token.clone();
				},

				_								=> {}	// carry on normally
			}

			//	get the next character from the file or None at EOF
			let next = self.next_char();

			//	at end of file if the reader vector length > 1 then pop it,
			//	change the state to BASE and keep going.  If reader vector
			//	length is == 1 then return the EOF token
			if next == None{
				if self.reader.len() > 1{
					self.reader.pop();
					self.tokenizer_state = TokenizerStates::BASE;
					continue;
				}else{
					self.token.token_type = TokenType::EOF;
					self.token.line_number = self.line_number;
					self.token.token_category = TokenCategory::Misc;
					return self.token.clone();
				}
			}

			//  if not at end of file or ERR then get the character and process it according
			//	to the current state
			let c = next.unwrap();
			//println!("Char = {} State = {}", next.unwrap(), self.tokenizer_state);
            match self.tokenizer_state{
				TokenizerStates::BASE 					=> self.state_BASE(c),
				TokenizerStates::BOT					=> self.state_BOT(c),
				TokenizerStates::OPERATOR				=> self.state_OPERATOR(c),
				TokenizerStates::KEYWORD_OR_ID 			=> self.state_KEYWORD_OR_ID(c),
				TokenizerStates::NUMBER					=> self.state_NUMBER(c),
				TokenizerStates::REAL_NUMBER			=> self.state_REAL_NUMBER(c),
				TokenizerStates::LINE_COMMENT			=> self.state_LINE_COMMENT(c),
				TokenizerStates::BLOCK_COMMENT			=> self.state_BLOCK_COMMENT(c),
				TokenizerStates::SINGLE_Q				=> self.state_SINGLE_Q(c),
				TokenizerStates::DOUBLE_Q				=> self.state_DOUBLE_Q(c),
				_ => {}
			}
		}
    }

	fn value_no_white_space(&mut self, c : char){
		match c{
			'\n' | '\r'| '\t' | ' ' => return,
			_ => {self.token.token_value.push(c)},
		}
	}

	fn last_no_white_space(&mut self, c : char){
		match c{
			'\n' | '\r'| '\t' | ' ' => return,
			_ => {self.last_char.push(c)},
		}
	}

	//	Waiting here for the first non white-space character.  When it is found,
	//	save it, change the state to BOT (begin of token), initialize the token
	//	to return.
	fn state_BASE(&mut self, c : char){
		match c{
			'\n' | '\r'| '\t' | ' ' => {},
			_ => {
				self.last_no_white_space(c);
				self.tokenizer_state = TokenizerStates::BOT;
				self.token = Token::new();
				//  remember the line that the first character of the token is on
				self.token.line_text = self.line.trim().to_string();
			}
		}
	}

	//	Capture the first character of the token.  The token has been iniitialized and
	//  the line the token is on has been captured.
    fn state_BOT(&mut self, c : char){
		if self.cli.is_debug_bit(TRACE_TOKENIZER){println!("state_BOT: '{}'", c);}
		self.value_no_white_space(c);
		self.tokenizer_state = TokenizerStates::EOT;

		//  assume that we're looking at a single character token (if not we change
		//	below)
		self.token.token_value = c.to_string();
        match c{
			'\t' | '\n' | '\r'  => {}		// *leading* white space

  			'a'..='z' |
			'A'..='Z' |
			':'				=> self.tokenizer_state = TokenizerStates::KEYWORD_OR_ID,
			'0'..='9' 		=> self.tokenizer_state =TokenizerStates::NUMBER,

			'"' 			=> {self.token.token_value.clear(); self.tokenizer_state =TokenizerStates::DOUBLE_Q},
			'\''			=> {self.token.token_value.clear(); self.tokenizer_state =TokenizerStates::SINGLE_Q},
			_ =>{
				//	If we're not looking at a potential ID, Number or String then push back
				//	this first character and handle operators
				self.last_char.push(c);
				self.tokenizer_state = TokenizerStates::OPERATOR;
			}
		}
	}

	//	Dealing with operators is different than IDs, or number or strings because
	//	we have to make sure we can tell the difference between, for example, + and ++
	fn state_OPERATOR(&mut self, c : char){
		if self.cli.is_debug_bit(TRACE_TOKENIZER){println!("state_OPERATOR: '{}'", c);}

		//	Assume we figure out the actual operator here
		self.tokenizer_state = TokenizerStates::EOT;

		match c{
			//	we need to decode the operators somewhere so here is as good a place as any
			//	some of the ops are multi character
			'='		=>	if self.op_look_ahead(c,'='){
							self.token.token_type= TokenType::EQ;
							self.token.token_value = "==".to_string();
						}else{
							self.token.token_type= TokenType::ASG_EQ;
						},
			'>'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = ">=".to_string();
							self.token.token_type= TokenType::GE;
						}else{
							self.token.token_type= TokenType::GT;
						},
			'<'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "<=".to_string();
							self.token.token_type= TokenType::LE;
						}else{
							self.token.token_type= TokenType::LT;
						},
			'!'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "!=".to_string();
							self.token.token_type= TokenType::NE;
						}else{
							self.token.token_type= TokenType::DAMNIT;
						},
			'|'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "|=".to_string();
							self.token.token_type= TokenType::ASG_OR_EQ;
						}else{
							if self.op_look_ahead(c,'|'){
								self.token.token_value = "||".to_string();
								self.token.token_type= TokenType::LOR;
							}else{
								self.token.token_type= TokenType::BITWISE_OR;
							}
						},
			'&'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "&=".to_string();
							self.token.token_type= TokenType::ASG_AND_EQ;
						}else{
							if self.op_look_ahead(c,'&'){
								self.token.token_value = "&&".to_string();
								self.token.token_type= TokenType::LAND;
							}else{
								self.token.token_type= TokenType::BITWISE_AND;
							}
						},

			'+'		=>{
						if self.cli.is_debug_bit(TRACE_TOKENIZER){println!(".... looking for '+='");}
						if self.op_look_ahead(c,'='){
							self.token.token_value = "+=".to_string();
							self.token.token_type= TokenType::ASG_ADD_EQ;
						}else{
							if self.cli.is_debug_bit(TRACE_TOKENIZER){println!(".... looking for '++'");}
							if self.op_look_ahead(c,'+'){
								if self.cli.is_debug_bit(TRACE_TOKENIZER){println!(".....PP found");}
								self.token.token_value = "++".to_string();
								self.token.token_type= TokenType::PP;
							}else{
								if self.cli.is_debug_bit(TRACE_TOKENIZER){println!("....ADD found");}
								self.token.token_type= TokenType::ADD;
							}
						}
					},

			'-'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "-=".to_string();
							self.token.token_type= TokenType::ASG_SUB_EQ;
						}else{
							if self.op_look_ahead(c,'-'){
								self.token.token_value = "--".to_string();
								self.token.token_type= TokenType::MM;
							}else{
								self.token.token_type= TokenType::SUB;
							}
						},

			'/'		=>	if self.op_look_ahead(c,'/'){
							self.token.token_type = TokenType::LINE_COMMENT;
							self.tokenizer_state = TokenizerStates::LINE_COMMENT;
							self.token.token_value.pop();
						}else{
							//println!("state_BOT found '/' lookng for begin of block comment");
							if self.op_look_ahead(c,'*'){
								//println!("state_BOT look_ahead found '*' begin block comment");
								self.token.token_type = TokenType::BLOCK_COMMENT;
								self.tokenizer_state = TokenizerStates::BLOCK_COMMENT;
								self.token.token_value.pop();
							}else{
								if self.op_look_ahead(c,'='){
									self.token.token_type= TokenType::ASG_DIV_EQ;
								}else{
									self.token.token_value = "/".to_string();
									self.token.token_type= TokenType::DIV;
								}
							}
						},
			'*'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "*=".to_string();
							self.token.token_type= TokenType::ASG_MUL_EQ;
						}else{
							self.token.token_type= TokenType::MUL;
						},
			'%'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = "%=".to_string();
							self.token.token_type= TokenType::ASG_MOD_EQ;
						}else{
							self.token.token_type= TokenType::MOD;
						},
			'.'		=>	if self.op_look_ahead(c,'='){
							self.token.token_value = ".=".to_string();
							self.token.token_type= TokenType::ASG_APPEND_EQ;
						}else{
							self.token.token_type= TokenType::CONCAT;
						},

			'('		=> self.token.token_type= TokenType::LPAREN,
			')'		=> self.token.token_type= TokenType::RPAREN,
			'{'		=> self.token.token_type= TokenType::LBRACE,
			'}'		=> self.token.token_type= TokenType::RBRACE,
			'['		=> self.token.token_type= TokenType::LBRACKET,
			']'		=> self.token.token_type= TokenType::RBRACKET,
			';'		=> self.token.token_type= TokenType::SEMI,
			','		=> self.token.token_type= TokenType::COMMA,
			'#'		=> self.token.token_type= TokenType::LENGTH_OF,
			_		=> {println!("Tokenizer state_OPERATOR unknown character: <{}>",c);}       
        }

		if self.cli.is_debug_bit(TRACE_TOKENIZER) {println!("state_BOT: token '{}' type: {}",self.token.token_value, self.token.token_type);}

    }




	//	the first character seen was a letter, so we know we are collecting either
	//	an ID (i.e. something that isn't a keyword) or a keyword (some verb we know about).
	//	keywords and ids can contain upper or lower case characters or digits.  Any other
	//	character will terminate the state and the token is the id or keyword found.  Embedded
	//	white space is not allowed
    fn state_KEYWORD_OR_ID(&mut self, c : char){
        match c{
			'a'..='z' |  'A'..='Z' | '0'..='9' | '_' | ':'  =>  self.value_no_white_space(c),
            _ =>   {
						//  Save the current character as it isn't part of the current token
						//  and set the state to EOT (end of token)
						self.last_no_white_space(c);
						self.tokenizer_state =TokenizerStates::EOT;
						self.token.token_type= TokenType::KEYWORD_OR_ID;
					},           
        }
    }

	//	The first character found was a digit, now collect the rest of the number
	fn state_NUMBER(&mut self, c : char){
        match c{
			'0'..='9'  =>  {self.value_no_white_space(c)},		//  number is still an integer
			'.'		   =>  {self.value_no_white_space(c); self.tokenizer_state = TokenizerStates::REAL_NUMBER}
			'\n' | '\r'| '\t' | ' ' =>  {},
            _ 		   =>  {self.last_no_white_space(c); self.tokenizer_state = TokenizerStates::EOT; self.token.token_type= TokenType::INTEGER},
        }
    }

	//	we started with digit and then saw a "." so we know we're capturing a
	//	floating point number
	fn state_REAL_NUMBER(&mut self, c : char){
        match c{
			'0'..='9'  =>  {self.value_no_white_space(c)},		//  this will be the fraction part of the float
			'.'		   =>  {self.error_text = "Syntax err:  extra '.' found".to_string(); self.tokenizer_state = TokenizerStates::ERR}
			'\n' | '\r'| '\t' | ' ' => {},
			_ 		   =>  {self.last_no_white_space(c); self.tokenizer_state = TokenizerStates::EOT; self.token.token_type= TokenType::FLOAT},
        }
    }

	//	we started with a line comment so characdters until end of line
	fn state_LINE_COMMENT(&mut self, c : char){
        match c{
			'\n' | '\r' => {self.tokenizer_state = TokenizerStates::EOT; self.token.token_type= TokenType::LINE_COMMENT},
			_ 		   =>  self.value_no_white_space(c),
        }
    }

	//	we started with /* collect value characters until we see */
	fn state_BLOCK_COMMENT(&mut self, c : char){
		//if self.cli.is_debug_bit(TRACE_TOKENIZER) {println!("state_BLOCK_COMMENT: token '{}' type: {}",self.token.token_value, self.token.token_type);}

		if c == '*'{
			//println!("state_BLOCK_COMMENT: found '*' looking ahead for '/'");
			if self.op_look_ahead(c,'/'){
				//println!("end block comment");
				self.tokenizer_state = TokenizerStates::EOT;
				self.token.token_type= TokenType::BLOCK_COMMENT;
				self.token.token_category=TokenCategory::Comment;
				return;
			}else{
				//println!("state_BLOCK_COMMENT '/' not found");				
			}
		}

		self.token.token_value.push(c);
		//println!("current state: {}", self.tokenizer_state);
    }

	//	We are collecting a string defined by "'" characters.
	fn state_SINGLE_Q(&mut self, c : char){
        match c{
			'\''		=> 	{
								self.tokenizer_state = TokenizerStates::EOT;
								self.token.token_type= TokenType::STRING;
							},
			_ 		   	=>  self.token.token_value.push(c),
        }
    }

	//	We are collecting a string defined by '"" characters.
	fn state_DOUBLE_Q(&mut self, c : char){
		//eprintln!("======== '{}'",c);
        match c{
			'"'			=> 	{
								self.tokenizer_state = TokenizerStates::EOT;
								self.token.token_type= TokenType::STRING;
							},
			'\\'		=>  {
								let lahead = self.next_char().unwrap();
								match lahead{
									't' => self.token.token_value.push('\t'),
									'n' => self.token.token_value.push('\n'),
									'r'	=> self.token.token_value.push('\r'),
									'\\' => self.token.token_value.push('\\'),
									_   => {
										self.token.token_value.push(c);
										self.token.token_value.push(lahead);
									},

								}
							},
			_ 		   	=>  self.token.token_value.push(c),
        }
    }

}
