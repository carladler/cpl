# CPL Programming Language

CPL is a simple text hacking language.  It is intended as a "student" project
to help me learn the Rust programming language.

## Description

CPL is a "C-like" language supporting constructs that you may find helpful
in simple text hacking applications.  It is loosley typed with variables
taking on whatever type of data is assigned to them.  The CPL program compiles
and runs CPL programs.

Please read (or at least attempt to read) the file:  cpldoc/cpl.pdf.  There is 
a lot there that might help explain what this thing does.

## Getting Started

### Dependencies

To build CPL, install Rust on your machine (https://www.rust-lang.org).  I've
built it on both Windows and Mac OS.

### Installing

After installing Rust, download the source code from Git into a directory
structure with "cpl" as the top level directory.  Then use:
```
cargo build
```
to construct the compiler.

### Executing program

Assuming the build compiles without errors you can then run any of the
CPL programs in the cpltests directory or you can run the complete test
script.  For example to run one of the tests:

```
cargo run allcplcode/cpltests/cpltest_pass_by_reference.cpl
```

and to run the entire suite of tests (on the Mac)

```
zsh aa_test_scripts/runall.sh
```

You can also run the compiler outside of Rust by directly executing
main (again, on the Mac). 

```
./target/debug/main allcplcode/cpltests/cpltest_pass_by_reference.cpl
```
If you build using the -r switch you will find the executable in
target/release:

```
./target/release/main allcplcode/cpltests/cpltest_pass_by_reference.cpl
```

In a Windows environment you will have to futz with test scripts to
use the Windows shell rather than zsh.

## Basic Syntax

CPL has the following grammatical constructs:

### Statements

```
Assignment
	target = <expression>
	target <op> <expression> (where <op> is, for example, +=)

If/Else
	if <condition> {...}else{...}

Eval/When/Otherwise
	eval <object>
		when <expression1> {...}  (when expression1 evaluates to <object> execute {...})
		when <expression2> {...}
		otherwise {...} (when none of the when expressions evaluate to <object>)

While
	while <condition>{...}

Loop
	loop{...}

Foreach
	foreach <target> <array> {...}

Break
	break
	break <depth expression>

Continue
Return
	return <expression>

Function Call
```

### Operators

```
"+", "-", "*", "/", "%", ".", ">", "<", "<=", ">=", "!=", "=="
```

```
"%" means modulo
"." means concat
```

### Assignment Operators

```
"+=", "-=", "*=", "/=", "%=", ".="
```

### Data Types

```
string: "this is a string"
number: 100, 100.10
bool: true, false
array = [1,2,3];
dictionary = {{"one",1},"two",2}}  
```

### Builtin Functions

```
I/O Functions:
	Fopen
	Fread/Freadln
	Freadln
	Fwrite / Fwriteln
	Feof

Collections
	Push
	Pop
	Contains
	Insert
	Delete
	Append
	Keys
	Sort

Strings
	Locate
	Substr
	Regex
	Match
	Capture
	Split
	Replace

Miscellaneous
	Length (depricated in favor of #<id>)
	GetType
```


## Help

I'm more than happy to help.  Send me a description of your issue,
any output from the compiler occurs and the CPL program you trying
to run.

There are things that a) might be wrong; or b) might not be supported
yet.

## Authors

It's just me so far:

Carl Adler
cpl@aequis.org

I have a web site that doesn't have much to do with CPL but here it
is anyway:  aequis.org (prefix with https://)

## Version History

* 1.0
	* Initial Release

## License

There are no licenses associated with this work.  You can do whatever you
want with it (including, and in particular, ignore it).  Please don't
use it to hurt any humans or animals.  And if you say anything about it
elsewhere, please mention my name and contact details (unless of course
you are bashing it, in which case, just keep it to yourself).

## Acknowledgments

This readme uses the template from:

* [[awesome-readme](https://github.com/matiassingers/awesome-readme)

In the process of writing this I've used much of the material from
the Rust home: https://www.rust-lang.org as well as stackoverflow:
https://stackoverflow.com.  There are some very smart and pleasent
people in the Rust community and I highly recommend interacting with
them.