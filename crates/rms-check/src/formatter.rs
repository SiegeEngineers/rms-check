use crate::{
    parser::{Atom, Parser},
    wordize::Word,
};
use codespan::{FileMap, FileName};
use std::iter::Peekable;

#[derive(Default)]
pub struct Formatter<'atom> {
    indent: u32,
    needs_indent: bool,
    result: String,
    prev: Option<Atom<'atom>>,
}

impl<'atom> Formatter<'atom> {
    fn newline(&mut self) {
        self.result.push_str("\r\n");
        self.needs_indent = true;
    }
    fn maybe_indent(&mut self) {
        if self.needs_indent {
            for _ in 0..self.indent {
                self.result.push(' ');
            }
            self.needs_indent = false;
        }
    }
    fn text(&mut self, text: &str) {
        self.maybe_indent();
        self.result.push_str(text);
    }

    fn command<'w>(&mut self, name: &Word<'w>, args: &[Word<'w>], is_block: bool) {
        self.text(name.value);
        for arg in args {
            self.result.push(' ');
            self.text(arg.value);
        }
        if is_block {
            self.result.push(' ');
        } else {
            self.newline();
        }
    }

    fn section<'w>(&mut self, name: &Word<'w>) {
        if let Some(_) = self.prev {
            self.newline();
        }
        self.text(name.value);
        self.newline();
    }

    fn block<I>(&mut self, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'atom>>,
    {
        use Atom::*;
        let is_end = |atom: &Atom<'_>| match atom {
            CloseBlock(_) => true,
            _ => false,
        };

        let mut commands = vec![];
        let mut longest = 0;
        for atom in input.by_ref().take_while(|atom| !is_end(atom)) {
            longest = match &atom {
                Command(cmd, _) => longest.max(cmd.value.len()),
                _ => longest,
            };
            commands.push(atom);
        }
        self.text("{");
        self.newline();
        self.indent += 2;
        for atom in commands {
            match atom {
                Command(name, args) => {
                    self.text(name.value);
                    if !args.is_empty() {
                        for _ in 0..(longest - name.value.len()) {
                            self.result.push(' ');
                        }
                        for arg in args {
                            self.result.push(' ');
                            self.text(arg.value);
                        }
                    }
                    self.newline();
                }
                _ => (),
            }
        }
        self.indent -= 2;
        self.text("}");
        self.newline();
        input
    }

    pub fn format(mut self, input: impl Iterator<Item = Atom<'atom>>) -> String {
        use Atom::*;
        let mut input = input.peekable();
        loop {
            let atom = match input.next() {
                Some(atom) => atom,
                _ => break,
            };

            match &atom {
                Section(name) => self.section(name),
                Command(name, args) => {
                    let is_block = if let Some(OpenBlock(_)) = input.peek() {
                        true
                    } else {
                        false
                    };
                    self.command(name, args, is_block);
                }
                OpenBlock(_) => {
                    input = self.block(input);
                }
                _ => (),
            }

            self.prev = Some(atom);
        }

        self.result
    }
}

pub fn format(source: &str) -> String {
    let f = FileMap::new(FileName::virtual_("format.rms"), source);
    let parser = Parser::new(&f);
    Formatter::default().format(parser.map(|(a, _)| a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_section() {
        assert_eq!(
            format("<PLAYER_SETUP> <OBJECTS_GENERATION>"),
            "<PLAYER_SETUP>\r\n\r\n<OBJECTS_GENERATION>\r\n"
        );
    }

    #[test]
    fn command_group() {
        assert_eq!(
            format("create_terrain GRASS3 { base_terrain DESERT border_fuzziness 5 }"),
            "create_terrain GRASS3 {\r\n  base_terrain     DESERT\r\n  border_fuzziness 5\r\n}\r\n"
        );
    }
}
