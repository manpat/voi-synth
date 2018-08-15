use std::collections::HashMap;
use failure::Error;

use voi_synth::{
	Context as SynthContext,
	context::EvaluationContext as SynthEvaluationContext,
	node::Input as SynthInput,
	Synth,
	Buffer as SynthBuffer,
	NodeContainer,
	NodeID, synth::StoreID
};

pub type LispResult<T> = Result<T, Error>;

pub fn evaluate(ctx: &mut SynthContext, input: &str) -> LispResult<()> {
	use std::iter::once;

	let comment_free_input = input.lines()
		.map(|l| l.split(';').next().unwrap())
		.flat_map(|l| l.chars().chain(once('\n')) )
		.collect::<String>();

	let top_level_exprs = ExprReader::new(&comment_free_input).parse_toplevel()?;

	let synth = evaluate_synth(ctx, top_level_exprs)?;
	ctx.push_synth(synth).unwrap();

	Ok(())
}



#[derive(Clone, Debug)]
enum SExpression<'a> {
	Identifier(&'a str),
	Number(f32),
	List(Vec<SExpression<'a>>),
}

use self::SExpression::*;

impl<'a> SExpression<'a> {
	fn expect_ident(self) -> LispResult<&'a str> {
		match self {
			Identifier(s) => Ok(s),
			Number(x) => bail!("Expected identifier, got number: {}", x),
			List(v) => bail!("Expected identifier, got list: ({:?})", v),
		}
	}
}




#[derive(Clone, Debug)]
enum EvalResult {
	Constant(f32),
	SynthNode(SynthInput),
	// Function(),
}

impl EvalResult {
	fn expect_constant(self) -> LispResult<f32> {
		match self {
			EvalResult::Constant(f) => Ok(f),
			EvalResult::SynthNode(n) => bail!("Expected constant value, got node: {:?}", n),
		}
	}
	fn to_input(self) -> LispResult<SynthInput> {
		match self {
			EvalResult::Constant(f) => Ok(f.into()),
			EvalResult::SynthNode(n) => Ok(n),
		}
	}
	fn expect_node_id(self) -> LispResult<NodeID> {
		use self::SynthInput::*;

		match self {
			EvalResult::Constant(f) => bail!("Expected synth node, got constant: {}", f),
			EvalResult::SynthNode(n) => match n {
				Literal(l) => bail!("Expected synth node, got Literal: {}", l),
				Node(n_id) => Ok(n_id),
				Store(s_id) => bail!("Expected synth node, got Store: {:?}", s_id),
			},
		}
	}
	fn expect_store_id(self) -> LispResult<StoreID> {
		use self::SynthInput::*;

		match self {
			EvalResult::Constant(f) => bail!("Expected synth store, got constant: {}", f),
			EvalResult::SynthNode(n) => match n {
				Literal(l) => bail!("Expected synth store, got Literal: {}", l),
				Node(n_id) => bail!("Expected synth store, got Node: {:?}", n_id),
				Store(s_id) => Ok(s_id),
			},
		}
	}
}

impl Into<EvalResult> for f32 {
	fn into(self) -> EvalResult { EvalResult::Constant(self) }
}

impl Into<EvalResult> for SynthInput {
	fn into(self) -> EvalResult { EvalResult::SynthNode(self) }
}

impl Into<EvalResult> for NodeID {
	fn into(self) -> EvalResult { EvalResult::SynthNode(self.into()) }
}

impl Into<EvalResult> for StoreID {
	fn into(self) -> EvalResult { EvalResult::SynthNode(self.into()) }
}




macro_rules! ensure_args {
    ($func:expr, $list:ident == $count:expr) => {{
    	ensure!($list.len() == $count,
    		"'{}' function requires {} arguments, {} received",
    		$func, $count, $list.len())
    }};

    ($func:expr, $list:ident >= $count:expr) => {{
    	ensure!($list.len() >= $count,
    		"'{}' function requires at least {} arguments, {} received",
    		$func, $count, $list.len())
    }};
}



fn evaluate_synth<'a>(ctx: &mut SynthContext, top_level: Vec<SExpression<'a>>) -> LispResult<Synth> {
	let mut ctx = EvaluationContext::new(ctx);

	for sexpr in top_level {
		if let SExpression::List(mut list) = sexpr {
			if list.is_empty() {
				bail!("Tried to evaluate an empty list");
			}

			let func_name = list.remove(0).expect_ident()?;

			match func_name {
				"let" => {
					ensure_args!(func_name, list == 2);

					let ident = list.remove(0).expect_ident()?;
					let value = ctx.evaluate_sexpr(list.remove(0))?;

					ctx.let_bindings.insert(ident, value);
				}

				"gain" => {
					ensure_args!(func_name, list == 1);
					let gain = ctx.evaluate_sexpr(list.remove(0))?.expect_constant()?;
					ctx.synth.set_gain(gain);
				}

				"output" => {
					ensure_args!(func_name, list == 1);

					let node_id = ctx.evaluate_sexpr(list.remove(0))?.expect_node_id()?;
					ctx.synth.set_output(node_id);
				}

				"def-store" => {
					ensure_args!(func_name, list == 1);
					let ident = list.remove(0).expect_ident()?;
					let store = ctx.synth.new_value_store();
					ctx.let_bindings.insert(ident, store.into());
				}

				"store" => {
					ensure_args!(func_name, list == 2);
					let ident = ctx.evaluate_sexpr(list.remove(0))?;
					let value = ctx.evaluate_sexpr(list.remove(0))?;
					ctx.synth.new_store_write(ident.expect_store_id()?, value.to_input()?);
				}

				_ => {
					list.insert(0, SExpression::Identifier(func_name));
					ctx.execute_function(list)?;
				}
			}

		} else {
			bail!("Unexpected item at top level of synth definition: {:?}", sexpr);
		}
	}

	Ok(ctx.synth)
}




struct EvaluationContext<'a> {
	synth_context: &'a mut SynthContext,
	synth: Synth,

	let_bindings: HashMap<&'a str, EvalResult>,
}


impl<'a> EvaluationContext<'a> {
	fn new(synth_context: &'a mut SynthContext) -> Self {
		EvaluationContext {
			synth_context,
			synth: Synth::new(),

			let_bindings: HashMap::new(),
		}
	}

	fn execute_function(&mut self, mut list: Vec<SExpression<'a>>) -> LispResult<EvalResult> {
		use std::cell::RefCell;

		if list.is_empty() {
			bail!("Tried to evaluate an empty list");
		}

		let func_name = list.remove(0).expect_ident()?;

		match func_name {
			"*" => {
				ensure_args!(func_name, list >= 2);

				let a = self.evaluate_sexpr(list.remove(0))?.to_input();
				let r_self = RefCell::new(self);

				// TODO: take advantage of associativity
				let res = list.into_iter()
					.map(|expr| r_self.borrow_mut().evaluate_sexpr(expr)?.to_input())
					.fold(a, |a, e| {
						Ok(r_self.borrow_mut().synth.new_multiply(a?, e?).into())
					});

				Ok(res?.into())
			}

			"+" => {
				ensure_args!(func_name, list >= 2);

				let a = self.evaluate_sexpr(list.remove(0))?.to_input();
				let r_self = RefCell::new(self);

				// TODO: take advantage of associativity
				let res = list.into_iter()
					.map(|expr| r_self.borrow_mut().evaluate_sexpr(expr)?.to_input())
					.fold(a, |a, e| {
						Ok(r_self.borrow_mut().synth.new_add(a?, e?).into())
					});

				Ok(res?.into())
			}

			"-" => {
				ensure_args!(func_name, list >= 2);

				let a = self.evaluate_sexpr(list.remove(0))?.to_input();
				let r_self = RefCell::new(self);

				let res = list.into_iter()
					.map(|expr| r_self.borrow_mut().evaluate_sexpr(expr)?.to_input())
					.fold(a, |a, e| {
						Ok(r_self.borrow_mut().synth.new_sub(a?, e?).into())
					});

				Ok(res?.into())
			}

			"mix" => {
				ensure_args!(func_name, list == 3);
				let a = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let b = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let mix = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_mix(a, b, mix).into())
			}

			"sin" | "sine" => {
				ensure_args!(func_name, list == 1);
				let freq = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_sine(freq).into())
			}

			"tri" | "triangle" => {
				ensure_args!(func_name, list == 1);
				let freq = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_triangle(freq).into())
			}

			"sqr" | "square" => {
				ensure_args!(func_name, list == 1);
				let freq = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_square(freq).into())
			}

			"saw" | "sawtooth" => {
				ensure_args!(func_name, list == 1);
				let freq = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_saw(freq).into())
			}

			"lp" | "lowpass" => {
				ensure_args!(func_name, list == 2);
				let cutoff = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let input = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_lowpass(cutoff, input).into())
			}

			"hp" | "highpass" => {
				ensure_args!(func_name, list == 2);
				let cutoff = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let input = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_highpass(cutoff, input).into())
			}

			"ar" | "env-ar" => {
				ensure_args!(func_name, list == 3);
				let attack = self.evaluate_sexpr(list.remove(0))?.expect_constant()?;
				let release = self.evaluate_sexpr(list.remove(0))?.expect_constant()?;
				let gate = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_env_ar(attack, release, gate).into())
			}

			"clamp" => {
				ensure_args!(func_name, list == 2);
				let input = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let lb = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				let ub = self.evaluate_sexpr(list.remove(0))?.to_input()?;
				Ok(self.synth.new_clamp(input, lb, ub).into())
			}

			"bake" => {
				ensure_args!(func_name, list >= 2);
				let sample_rate = self.synth_context.get_sample_rate();
				let samples = self.evaluate_sexpr(list.remove(0))?.expect_constant()? * sample_rate;
				let samples = samples as usize;

				ensure!(samples > 0, "You can't bake a synth to a zero length buffer");

				let mut synth = evaluate_synth(self.synth_context, list)?;
				let mut eval_ctx = SynthEvaluationContext::new(sample_rate);
				let mut eval_buffer = SynthBuffer::new(samples);

				synth.evaluate_into_buffer(&mut eval_buffer, &mut eval_ctx);
				let buffer_id = self.synth.new_buffer(eval_buffer.data);

				Ok(self.synth.new_sampler(buffer_id).into())
			}

			_ => bail!("Unknown function: '{}'", func_name),
		}
	}

	fn evaluate_sexpr(&mut self, sexpr: SExpression<'a>) -> LispResult<EvalResult> {
		match sexpr {
			List(v) => self.execute_function(v),
			Number(n) => Ok(EvalResult::Constant(n)),
			Identifier(i) => {
				self.let_bindings.get(&i)
					.cloned()
					.ok_or_else(|| format_err!("Unknown identifier: '{}'", i))
			}
		}
	}
}




#[derive(Copy, Clone, Debug)]
struct ExprReader<'a> {
	input: &'a str,
}

impl<'a> ExprReader<'a> {
	fn new(input: &str) -> ExprReader {
		ExprReader {input}
	}

	fn is_empty(&self) -> bool { self.input.is_empty() }

	fn peek(&self) -> LispResult<char> {
		self.input.chars()
			.next()
			.ok_or_else(|| format_err!("Hit end of input"))
	}

	fn expect(&mut self, c: char) -> LispResult<()> {
		self.skip_whitespace();
		let next = self.peek()?;

		if next != c {
			bail!("Unexpected character '{}', expected '{}'", next, c)
		}

		self.input = &self.input[next.len_utf8()..];
		Ok(())
	}

	fn skip_whitespace(&mut self) {
		self.input = self.input.trim_left();
	}

	fn parse_toplevel(&mut self) -> LispResult<Vec<SExpression<'a>>> {
		let mut top_level_exprs = Vec::new();

		self.skip_whitespace();

		while !self.is_empty() {
			top_level_exprs.push(self.parse_sexpression()?);
			self.skip_whitespace();
		}

		Ok(top_level_exprs)
	}

	fn parse_sexpression(&mut self) -> LispResult<SExpression<'a>> {
		if self.peek()? == '(' {
			let list = self.parse_list()?;
			Ok( List(list) )

		} else {
			let word = self.parse_word()?;

			if let Ok(f) = word.parse() {
				Ok( Number(f) )
			} else {
				Ok( Identifier(word) )
			}
		}
	}

	fn parse_word(&mut self) -> LispResult<&'a str> {
		self.skip_whitespace();

		let word_end = self.input
			.find(char::is_whitespace)
			.unwrap_or(self.input.len());

		let (word, rest) = self.input.split_at(word_end);
		self.input = rest;
		Ok(word)
	}

	fn parse_list(&mut self) -> LispResult<Vec<SExpression<'a>>> {
		let mut list_parser = self.list_parser()?;
		let mut ret = Vec::new();

		list_parser.skip_whitespace();

		while !list_parser.is_empty() {
			ret.push(list_parser.parse_sexpression()?);
			list_parser.skip_whitespace();
		}
		
		Ok(ret)
	}

	fn list_parser(&mut self) -> LispResult<ExprReader<'a>> {
		self.expect('(')?;

		let end = self.input
			.char_indices()
			.scan(1, |level, (pos, c)| {
				match c {
					'(' => { *level += 1 }
					')' => { *level -= 1 }
					_ => {}
				}

				Some((*level, pos))
			})
			.find(|(l, _)| *l == 0);

		if let Some((_, pos)) = end {
			let (list_str, rest) = self.input.split_at(pos);

			self.input = rest;
			self.expect(')')?;

			Ok(ExprReader::new(list_str))
		} else {
			bail!("Couldn't find end of the list");
		}
	}
}
