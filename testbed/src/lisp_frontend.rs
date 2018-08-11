// use voi_synth::*;
use failure::Error;

pub type LispResult<T> = Result<T, Error>;

pub struct Context {}

impl Context {
	pub fn new() -> Self {
		Context {}
	}

	pub fn evaluate(&mut self, input: &str) -> LispResult<()> {
		let mut reader = ExprReader::new(input);
		let mut top_level_exprs = Vec::new();

		reader.skip_whitespace();

		while !reader.is_empty() {
			top_level_exprs.push(reader.parse_sexpression()?);
			reader.skip_whitespace();
		}

		println!("{:?}", top_level_exprs);

		Ok(())
	}
}

#[derive(Copy, Clone, Debug)]
struct ExprReader<'a> {
	input: &'a str,
}

#[derive(Clone, Debug)]
enum SExpression<'a> {
	Identifier(&'a str),
	Number(f32),
	List(Vec<SExpression<'a>>),
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
		let next = self.input
			.char_indices()
			.skip_while(|(_, c)| c.is_whitespace())
			.next();

		if let Some((pos, _)) = next {
			self.input = &self.input[pos..];
		} else {
			self.input = &self.input[0..0];
		}
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

	fn parse_word(&mut self) -> LispResult<&'a str> {
		self.skip_whitespace();

		let word_end = self.input
			.char_indices()
			.skip_while(|(_, c)| !c.is_whitespace())
			.map(|(p, _)| p)
			.next();

		if let Some(word_end) = word_end {
			let (word, rest) = self.input.split_at(word_end);
			self.input = rest;
			Ok(word)
		} else {
			let word = self.input;
			self.input = &self.input[0..0];
			Ok(word)
		}
	}

	fn parse_sexpression(&mut self) -> LispResult<SExpression<'a>> {
		self.skip_whitespace();

		if self.peek()? == '(' {
			let list = self.parse_list()?;
			Ok( SExpression::List(list) )

		} else {
			let word = self.parse_word()?;

			if let Ok(f) = word.parse() {
				Ok( SExpression::Number(f) )
			} else {
				Ok( SExpression::Identifier(word) )
			}
		}
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
}


/*

(defstore feedback)

(let lfo (* (sin 6) (sin feedback)))

(let result
	(+	(sine (+ lfo 440))
		(triangle (+ lfo 220))))

(store feedback result)
(output result)

*/
