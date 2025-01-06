
use std::env;
use cli::*;
use parser::*;
use cplvar::*;
use exec::*;
//use builtin::*;
use codeframe::*;
use std::collections::HashMap;
use opcode::*;
use machineinstruction::*;
use std::time::SystemTime;
use std::time::Duration;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;



fn main() {
	let cl = env::args().collect();
	match CLI::new(&cl){
		None => return,
		Some(cli) => {
			if cli.is_debug_bit(SET_BACKTRACE){
				env::set_var("RUST_BACKTRACE", "1");
			}
			parse(&cli);
		}
	};
}

fn parse(cli : &CLI){
	let start_time = SystemTime::now();

	let parse_result : (bool, Option<FrameMap>);
	{
		let mut _parser = Parser::new(cli);
		parse_result = _parser.parse_and_gen();
	}

	if !parse_result.0{
		return;
	}

	match parse_result.1{
		Some(mut f) => {
			let mut arguments : Vec<CplVar> = Vec::new();
			let mut operand_stack = OperandStack::new(cli.is_runtime_warnings());
			let mut runtime_data = RuntimeData::new();
			let mut e = Executor::new(cli, &mut f, &mut arguments, &mut operand_stack, 0, &mut runtime_data);
			let rtn = e.exec();

			//	if the runtime arguments specify it, dump the runtime stats
			display_stats(cli, &runtime_data, start_time);

			match rtn.var{
				CplDataType::CplNumber(n) => if n.cpl_number as i32 != 0{
					println!("Program exiting with: {}", n.cpl_number);
					std::process::exit(n.cpl_number as i32); 
				},

				CplDataType::CplString(s) => {
					if s.cpl_string != "$$Synthetic$$"{
						println!("Program exit with: {}", s.cpl_string);
					}
				},

				CplDataType::CplUninitialized(_)=> {},
				
				_ => println!("Program exit with: {}", rtn),
			}
		},
		None => {},
	}
}

#[derive(Clone)]
struct Accumulator{
	accum_duration : Duration,
	accum_execution_count : u32,
	accum_average_duration : Duration,
}

impl Accumulator{
	fn new(accum_duration : Duration, accum_execution_count : u32, accum_average_duration : Duration) -> Accumulator{
		Accumulator{
			accum_duration : accum_duration,
			accum_execution_count : accum_execution_count,
			accum_average_duration : accum_average_duration,
		}
	}
}

#[derive(Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
struct InstructionKey{
	opcode : Opcode,
	opcode_mode : OpcodeMode,
	qual : String,
}

impl InstructionKey{
	fn to_key (instruction : &MachineInstruction, qual : String) -> InstructionKey{
		InstructionKey{
			opcode : instruction.opcode,
			opcode_mode : instruction.opcode_mode,
			qual : qual,
		}
	}

	fn to_key2 (opcode : Opcode, opcode_mode : OpcodeMode, qual : String) -> InstructionKey{
		InstructionKey{
			opcode : opcode,
			opcode_mode : opcode_mode,
			qual : qual,
		}
	}
}

fn dump_stats_to_csv(csv_file: &str, event_accumulator : &HashMap<InstructionKey,Accumulator>){

	let mut writer = BufWriter::new(File::create(csv_file).expect(&format!("--Unable to open {}--\n",csv_file)));

	let keys = event_accumulator.keys();

	// write header records
	writer.write_all(format!("OPCODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("MODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("QUAL,").as_bytes()).expect("Unable to write data");

	writer.write_all(format!("TOTAL ELAPSED,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("CALLS,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");

	for key in keys{
		writer.write_all(format!("{},",key.opcode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",key.opcode_mode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",key.qual).as_bytes()).expect("Unable to write data");

		writer.write_all(format!("{},",event_accumulator.get(key).unwrap().accum_duration.as_micros()).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",event_accumulator.get(key).unwrap().accum_execution_count).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");
	}
}


//	pass 1 computes the accumulated cpu usage for each instruction and returns the
//	total accumulated elapsed time across all opcodes
fn pass1(runtime_data : &RuntimeData, event_accumulator : &mut HashMap<InstructionKey,Accumulator>) -> Duration{
	let mut  total_accum_elapsed : u64 = 0;

	for event in &runtime_data.events{
		//	Fetch the instruction record
		let accum_event = event_accumulator.get(&InstructionKey::to_key(&event.machine_instruction, event.qual.clone()));
		match accum_event {
			//	If this is the first one we've seen for this opcode then initialized
			None => {
				event_accumulator.insert(InstructionKey::to_key(&event.machine_instruction, event.qual.clone()), Accumulator::new(event.elapsed,1,event.elapsed));
			}

			//	Otherwise accumulate and compute new average
			Some(e) =>{
				let current_accum = e.accum_duration;
				let current_count = e.accum_execution_count;

				//	add this duration to the total
				total_accum_elapsed += event.elapsed.as_micros() as u64;

				//	accumulate the duration of this event
				let accum_micros = current_accum.as_micros() + event.elapsed.as_micros();
				let accumulated_duration = Duration::from_micros(accum_micros as u64);

				//	accumulate the number events for this opcode
				let opcode_count = current_count + 1;

				//	recompute the averate
				let average_micros = accum_micros as f64 / opcode_count as f64;
				let average_nanos = (average_micros * 1000.0) as u64;
				let average_duration = Duration::from_nanos(average_nanos);
				event_accumulator.insert(InstructionKey::to_key(&event.machine_instruction, event.qual.clone()), Accumulator::new(accumulated_duration, opcode_count, average_duration));
			}
		}
	}
	return Duration::from_micros(total_accum_elapsed);
}

//	pass2 computes the percentage of the whole for each opcode and sorts it
//	by highest to lowest usage
fn pass2(event_accumulator : &HashMap<InstructionKey,Accumulator>, total_accum_elapsed : Duration, duration_list : &mut Vec<(InstructionKey, Accumulator, f64)>){
	let instruction_keys = event_accumulator.keys();
	for instruction_key in instruction_keys{
		let opcode_accum = event_accumulator.get(instruction_key).unwrap();
		let percent = ((opcode_accum.accum_duration.as_micros() * 100) as f64) / total_accum_elapsed.as_micros() as f64;
		duration_list.push((instruction_key.clone(), opcode_accum.clone(), percent));
	}
}

fn display_stats(cli : &CLI, runtime_data : &RuntimeData, start_time : SystemTime){
	//	if neither the -d27 switch or the -p switch is present then we're done
	if !cli.is_debug_bit(DUMP_PERFORMANCE_STATS) && cli.get_performance_output_file() == None{
		return;
	}

	//	accum is the  accumulated durations, number of calls and the average time

	let mut event_accumulator : HashMap<InstructionKey,Accumulator> = HashMap::new();
	let total_accum_elapsed = pass1(runtime_data, &mut event_accumulator);

	let performance_csv_file = cli.get_performance_output_file();
	match performance_csv_file{
		None => {}
		Some(ref f) => dump_stats_to_csv(f, &event_accumulator),
	}

	//	if user only wanted the csv file then we're done
	if !cli.is_debug_bit(DUMP_PERFORMANCE_STATS){return};

	let mut duration_list : Vec<(InstructionKey, Accumulator, f64)> = Vec::new();
	pass2(&event_accumulator, total_accum_elapsed, &mut duration_list);

	//	Now we need to create an inversion:
	let mut percent_list : Vec<(u64, InstructionKey)> = Vec::new();

	for duration in &duration_list{
		let instruction_key = InstructionKey::to_key2(duration.0.opcode, duration.0.opcode_mode, duration.0.qual.clone());
		let percent = (duration.2*100000.0) as u64;
		percent_list.push((percent, instruction_key));
	}

	percent_list.sort();
	percent_list.reverse();
	
	eprintln!("\n*** Internal Runtime Statistics ***");
	eprintln!("Total run time: {:?}", start_time.elapsed().unwrap());
	eprintln!("Total accumulated Elapsed time: {:?}",total_accum_elapsed);
	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","opcode", "mode", "qual", "elapsed", "calls", "average", "percent");
	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","------", "----", "----", "-------", "-----", "-------", "-------");

	for percent in percent_list{
		let accum_data = event_accumulator.get(&percent.1).unwrap();

		let opcode_txt 						= format!("{}"		,percent.1.opcode);
		let opcode_mode_txt					= format!("{}"		,percent.1.opcode_mode);
		let qual_txt						= format!("{}"		,percent.1.qual);
		let accum_duration_txt 				= format!("{:?}"	,accum_data.accum_duration);
		let accum_execution_count_txt 		= format!("{}"		,accum_data.accum_execution_count);
		let accum_average_duration_txt 		= format!("{:?}"	,accum_data.accum_average_duration);

		let actual_percent = percent.0 as f64 / 100000.0;

		let percent_txt 					= format!("{:2.1}%"	,actual_percent);

		eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9}"
				,opcode_txt
				,opcode_mode_txt
				,qual_txt
				,accum_duration_txt
				,accum_execution_count_txt
				,accum_average_duration_txt
				,percent_txt
		);
	}
}