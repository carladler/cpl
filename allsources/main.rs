
use std::env;
use cli::*;
use parser::*;
use cplvar::*;
use exec::*;
//use builtin::*;
use codeframe::*;
//use opcode::*;
//use machineinstruction::*;
use std::time::SystemTime;
use std::time::Duration;
use runtimestats::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::cell::RefCell;
use names::*;


fn main() {
	let cl = env::args().collect();
	let names = RefCell::new(Names::new());

	//	make sure slot 0 has "TempOrUnknownName" in it
	names.borrow_mut().add("TempOrUnknownName");

	match CLI::new(&cl){
		None => return,
		Some(cli) => {
			if cli.is_debug_bit(SET_BACKTRACE){
				env::set_var("RUST_BACKTRACE", "1");
			}
			parse_and_run(&cli, names);
		}
	};
}




fn parse_and_run(cli : &CLI, names : RefCell<Names>){

	let parse_result : (bool, Option<FrameMap>);
	
	let mut _parser = Parser::new(cli, &names);
	parse_result = _parser.parse_and_gen();

	if !parse_result.0{
		return;
	}

	// let name_list = &names.borrow().names;
	// for name in name_list{
	// 	println!("name! {}",name);
	// }

	match parse_result.1{
		Some(mut frame_map) => {
			let mut arguments : Vec<CplVar> = Vec::new();
			let mut operand_stack = OperandStack::new(cli.is_runtime_warnings(), names.borrow().names.clone());
			let mut runtime_data = RuntimeData::new();
			
			// if cli.is_runtime_stats_enabled(){
			// 	runtime_data.events.reserve(50_500_000);
			// }

			let start_time = SystemTime::now();

			let names_list = &names.borrow().names.clone();
			
			let mut executor = Executor::new(cli, &mut frame_map, &mut arguments, &mut operand_stack, 0, &mut runtime_data, &names_list);
			let rtn = executor.exec();

			//	if neither the -d27 switch or the -p switch is present then we're done
			if cli.is_runtime_stats_enabled(){
				eprintln!("\nRuntime: {}", format_duration(start_time.elapsed().unwrap()));
				let total_accumulated_time : u64 = 0;
				let mut _event_count : usize = 0;
				display_stats(cli, executor.get_runtime_data(), total_accumulated_time);
			}else if cli.is_debug_bit(DISPLAY_RUNTIME){
				eprintln!("\nRuntime: {}", format_duration(start_time.elapsed().unwrap()));
			}

			executor.exit(&rtn);
		},
		None => {},
	}
}

fn display_stats(cli : &CLI, runtime_data: HashMap<InstructionKey,EventPayload>, _total_accumulated_time : u64){

	//	If the -p switch is specified then write the raw performance data to a
	//	csv file
	let performance_csv_file = cli.get_performance_output_file();
	match performance_csv_file{
		None => {}
		Some(ref f) => dump_stats_to_csv(f, &runtime_data),
	}

	//	if user only wanted the csv file then we're done
	if !cli.is_debug_bit(DUMP_PERFORMANCE_STATS){return};

	let mut total_instruction_count = 0;
	let mut total_accumulated_time : Duration = Default::default();

	let mut event_summaries : Vec<EventSummary> = Vec::new();

	//	Get all of the InstructionKey data from the runtime data hash
	let keys = runtime_data.keys();
	for key in keys.clone().into_iter(){
		let rd = runtime_data.get(key).unwrap();

		//	accumulate the number of instructions executed and the accumulated
		//	time for all of the different instructions that were executed
		total_instruction_count += rd.event_count;
		total_accumulated_time += rd.event_accum_duration;

		//	Compute the averate execution time for each of the instructions
		//	executed

		let avg = rd.event_accum_duration.as_nanos() as u64 / rd.event_count;
		let avg_execution_time = Duration::from_nanos(avg as u64);
		// eprintln!("============ qual {} duration {} count {} avg {} avg duration {:?}",key.qual, rd.event_accum_duration.as_nanos(), rd.event_count, avg, Duration::from_nanos(avg));

		//	add all of this to an event summary record
		let event_summary = EventSummary::new(&key, rd.event_accum_duration, rd.event_count, avg_execution_time);
		event_summaries.push(event_summary);
	}

	//	Now we have to do some more stuff
	let mut cooked_summaries: Vec<(u64, EventSummary)> = Vec::new();

	//	compute the percent of the accumulated time accrued by each of the
	//	instructions
	for raw_summary in event_summaries{
		let ratio_as_u64 = ((raw_summary.accum_duration.as_nanos() * 1000) / total_accumulated_time.as_nanos()) as u64;
		// let ratio : f64 = raw_summary.accum_duration.as_nanos() as f64 / total_accumulated_time.as_nanos() as f64;
		cooked_summaries.push((ratio_as_u64, raw_summary));
	}

	//	We want to display the stats by percent of total in descending order
	cooked_summaries.sort();
	cooked_summaries.reverse();

	eprintln!("\n*** Internal Runtime Statistics ***\n");

	eprintln!("Instruction Count: {:3.2}M", total_instruction_count as f64/1_000_000.0);
	eprintln!("Total Accumulated Time: {}", format_duration(total_accumulated_time));

	let rate_divisor = total_accumulated_time.as_nanos()/1_000_000_000;
	if rate_divisor > 0{
		let rate : f64 = (total_instruction_count / rate_divisor as u64) as f64;
		eprintln!("Execution Rate: {:3.2}M instructions/sec", rate/1_000_000.0);
	}

	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","opcode", "mode", "qual", "elapsed", "calls", "average", "percent");
	eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9} ","------", "----", "----", "-------", "-----", "-------", "-------");

	for cooked_summary in cooked_summaries{
		let opcode_txt 						= format!("{}"		,cooked_summary.1.opcode);
		let opcode_mode_txt					= format!("{}"		,cooked_summary.1.opcode_mode);
		let qual_txt						= format!("{}"		,cooked_summary.1.qual);
		let accum_duration_txt				= format!("{}"		,format_duration(cooked_summary.1.accum_duration));
		let accum_execution_count_txt		= format!("{}"		,cooked_summary.1.accum_execution_count);
		let accum_average_duration_txt		= format!("{}"		,format_duration(cooked_summary.1.accum_average_duration));
		let percent_txt						= format!("{:3.1}%"	,cooked_summary.0 as f64 / 10.0);
	
		// eprintln!("{0: <20} | {1: <8} | {2: <8} | {3: <12} | {4: <9} | {5: <9} | {6: <9}"
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

fn format_duration(duration : Duration) -> String{
	const ONE_MINUTE : u128 = 1000000000 * 60;
	const ONE_SECOND : u128 = 1000000000;
	const ONE_MILISECOND : u128 = 1000000;
	const ONE_MICROSECOND : u128 = 1000;

	let nano = duration.as_nanos();
	if nano > ONE_MINUTE {
		return format!("{:3.1}mins", nano as f64 / ONE_MINUTE as f64);
	}else if nano > ONE_SECOND{
		return format!("{:3.2}s", nano as f64 / ONE_SECOND as f64);
	}else if nano > ONE_MILISECOND{
		return format!("{:3.2}ms", nano as f64 / ONE_MILISECOND as f64);
	}else{
		return format!("{:3.2}µs", nano as f64 / ONE_MICROSECOND as f64);
	}
}

fn dump_stats_to_csv(csv_file: &str, runtime_data : &HashMap<InstructionKey, EventPayload>){

	let keys = runtime_data.keys();

	let mut writer = BufWriter::new(File::create(csv_file).expect(&format!("--Unable to open {}--\n",csv_file)));
	// write header records
	writer.write_all(format!("OPCODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("MODE,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("QUAL,").as_bytes()).expect("Unable to write data");

	writer.write_all(format!("TOTAL ELAPSED,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("CALLS,").as_bytes()).expect("Unable to write data");
	writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");

	for key in keys{
		let rd = runtime_data.get(&key).unwrap();
		writer.write_all(format!("{},",key.opcode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",key.opcode_mode).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",key.qual).as_bytes()).expect("Unable to write data");

		writer.write_all(format!("{},",rd.event_accum_duration.as_nanos()).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("{},",rd.event_count).as_bytes()).expect("Unable to write data");
		writer.write_all(format!("\n").as_bytes()).expect("Unable to write data");
	}
}
