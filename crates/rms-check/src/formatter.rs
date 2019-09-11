use crate::{
    parser::{Atom, Parser},
    wordize::Word,
};
use codespan::Files;
use std::iter::Peekable;

/// Keeps track of alignment widths for commands/attributes.
#[derive(Debug, Default, Clone, Copy)]
struct Width {
    /// The width of the widest command in a block.
    command_width: usize,
    /// The width of the widest first argument of any command in a block.
    arg_width: usize,
}

/// Formatting options.
///
/// ## Example
/// ```rust
/// use rms_check::{format, FormatOptions};
/// let opts = FormatOptions::default()
///     .tab_size(8)
///     .use_spaces(true)
///     .align_arguments(false);
/// let result = format("create_object SCOUT { number_of_objects 5 group_placement_radius 3 }", opts);
/// assert_eq!(result, "create_object SCOUT {\r
///         number_of_objects 5\r
///         group_placement_radius 3\r
/// }\r
/// ");
/// ```
#[derive(Debug, Clone)]
pub struct FormatOptions {
    tab_size: u32,
    use_spaces: bool,
    align_arguments: bool,
}

impl Default for FormatOptions {
    fn default () -> Self {
        Self {
            tab_size: 2,
            use_spaces: true,
            align_arguments: true,
        }
    }
}

impl FormatOptions {
    /// Set the size in spaces of a single tab indentation (default 2). This is only used if
    /// `use_spaces()` is enabled.
    pub fn tab_size(self, tab_size: u32) -> Self { Self { tab_size, ..self } }
    /// Whether to use spaces instead of tabs for indentation (default true).
    pub fn use_spaces(self, use_spaces: bool) -> Self { Self { use_spaces, ..self } }
    /// Whether to align arguments in a list of commands (default true).
    ///
    /// ## Example
    /// When enabled:
    /// ```rms
    /// create_object SCOUT {
    ///   number_of_objects   5
    ///   group_variance      5
    ///   terrain_to_place_on GRASS
    /// }
    /// ```
    /// When disabled:
    /// ```rms
    /// create_object SCOUT {
    ///   number_of_objects 5
    ///   group_variance 5
    ///   terrain_to_place_on GRASS
    /// }
    /// ```
    pub fn align_arguments(self, align_arguments: bool) -> Self { Self { align_arguments, ..self } }
}

#[derive(Debug, Default, Clone)]
pub struct Formatter<'atom> {
    options: FormatOptions,
    /// The current indentation level.
    indent: u32,
    /// Whether this line still needs indentation. A line needs indentation if no text has been
    /// written to it yet.
    needs_indent: bool,
    /// Command name and argument widths in a given context.
    widths: Vec<Width>,
    /// Whether we are inside a command block.
    inside_block: usize,
    /// The formatted text.
    result: String,
    /// The last-written atom.
    prev: Option<Atom<'atom>>,
}

impl<'atom> Formatter<'atom> {
    fn new(options: FormatOptions) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }

    /// Write a newline (Windows-style).
    fn newline(&mut self) {
        self.result.push_str("\r\n");
        self.needs_indent = true;
    }

    /// Indent the current line if it still needs it.
    fn maybe_indent(&mut self) {
        if self.needs_indent {
            if self.options.use_spaces {
                for _ in 0..self.indent * self.options.tab_size {
                    self.result.push(' ');
                }
            } else {
                for _ in 0..self.indent {
                    self.result.push('\t');
                }
            }
            self.needs_indent = false;
        }
    }

    /// Write some text to the current line.
    fn text(&mut self, text: &str) {
        self.maybe_indent();
        self.result.push_str(text);
    }

    /// Write a command.
    fn command<'w>(&mut self, name: &Word<'w>, args: &[Word<'w>], is_block: bool) {
        self.text(name.value);
        let Width { command_width, arg_width } = self.widths.last().cloned().unwrap_or_default();

        let mut arg_iter = args.iter().peekable();

        if self.options.align_arguments {
            // If we have any args, add padding spaces between the command name and arg1, and between
            // arg1 and arg2.
            // The rest is not handled right now since they are less frequent and it's not certain
            // that lining them up makes sense.
            if let Some(arg1) = arg_iter.next() {
                for _ in 0..command_width.saturating_sub(name.value.len()) {
                    self.result.push(' ');
                }

                self.result.push(' ');
                self.text(arg1.value);

                if arg_iter.peek().is_some() {
                    for _ in 0..arg_width.saturating_sub(arg1.value.len()) {
                        self.result.push(' ');
                    }
                }
            }
        }

        for arg in arg_iter {
            self.result.push(' ');
            self.text(arg.value);
        }

        if is_block {
            self.result.push(' ');
        } else {
            self.newline();
        }
    }

    /// Write a section header.
    fn section<'w>(&mut self, name: &Word<'w>) {
        if let Some(_) = self.prev {
            self.newline();
        }
        self.text(name.value);
        self.newline();
    }

    /// Write a command block. This reads atoms from the iterator until the end of the block, and
    /// writes both the command and any attributes it may contain.
    fn block<I>(&mut self, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'atom>>,
    {
        use Atom::*;
        let is_end = |atom: &Atom<'_>| match atom {
            CloseBlock(_) => true,
            _ => false,
        };

        self.inside_block += 1;

        let mut commands = vec![];
        let mut width = Width::default();
        let mut indent = 0;
        for atom in input.by_ref().take_while(|atom| !is_end(atom)) {
            width = match &atom {
                Command(cmd, args) => Width {
                    command_width: width.command_width.max(cmd.value.len() + indent * self.options.tab_size as usize),
                    arg_width: width.arg_width.max(args.get(0).map(|word| word.value.len()).unwrap_or(0)),
                },
                If(_, _) => {
                    indent += 1;
                    width
                }
                EndIf(_) => {
                    indent -= 1;
                    width
                }
                _ => width,
            };
            commands.push(atom);
        }
        self.text("{");
        self.newline();
        self.indent += 1;

        self.widths.push(width);
        let mut sub_input = commands.into_iter().peekable();
        while let Some(atom) = sub_input.next() {
            sub_input = self.write_atom(atom, sub_input);
        }
        self.widths.pop();

        self.inside_block -= 1;

        self.indent -= 1;
        self.text("}");
        self.newline();

        input
    }

    fn condition<I>(&mut self, cond: &Word<'_>, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'atom>>,
    {
        use Atom::*;

        self.text("if ");
        self.text(cond.value);
        self.newline();
        self.indent += 1;

        // reset command width so an if block within a command block
        // does not over-indent.
        let Width { command_width, arg_width } = self.widths.last().cloned().unwrap_or_default();
        self.widths.push(Width {
            command_width: command_width.saturating_sub(2),
            arg_width,
        });

        let mut depth = 1;
        let content: Vec<Atom<'atom>> = input
            .by_ref()
            .take_while(|atom| {
                match atom {
                    If(_, _) => depth += 1,
                    EndIf(_) => depth -= 1,
                    _ => (),
                }

                match atom {
                    EndIf(_) if depth == 0 => false,
                    _ => true,
                }
            })
            .collect();

        let mut sub_input = content.into_iter().peekable();
        while let Some(atom) = sub_input.next() {
            match atom {
                Atom::ElseIf(_, cond) => {
                    self.indent -= 1;
                    self.text("elseif ");
                    self.text(cond.value);
                    self.newline();
                    self.indent += 1;
                }
                Atom::Else(_) => {
                    self.indent -= 1;
                    self.text("else");
                    self.newline();
                    self.indent += 1;
                }
                _ => {
                    sub_input = self.write_atom(atom, sub_input);
                }
            }
        }

        self.widths.pop();

        self.indent -= 1;
        self.text("endif");
        self.newline();

        if self.inside_block == 0 {
            self.newline();
        }

        input
    }

    fn random<I>(&mut self, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'atom>>,
    {
        use Atom::*;

        self.text("start_random");
        self.newline();
        self.indent += 1;

        // reset command width so a start_random within a command block
        // does not over-indent.
        self.widths.push(Width::default());

        let mut null_branch = vec![];
        let mut branches = vec![];
        let mut depth = 1;
        for atom in input.by_ref() {
            match &atom {
                PercentChance(_, arg) if depth == 1 => {
                    branches.push((arg.clone(), vec![]));
                    continue;
                }
                StartRandom(_) => {
                    depth += 1;
                }
                EndRandom(_) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => (),
            };

            if branches.len() > 0 {
                branches.last_mut().unwrap().1.push(atom);
            } else {
                null_branch.push(atom);
            }
        }

        let has_simple_branches = branches.iter().all(|(_, stmts)| {
            if stmts.len() > 1 {
                return false;
            }
            if stmts.len() == 0 {
                return true;
            }
            match stmts[0] {
                Define(_, _) => true,
                Const(_, _, _) => true,
                // Include(_, _) => true,
                // IncludeDrs(_, _) => true,
                Command(_, _) => true,
                _ => false,
            }
        });

        if has_simple_branches {
            let longest = branches.iter().fold(0, |acc, (chance, _)| {
                acc.max(format!("percent_chance {}", chance.value).len())
            });
            for (chance, mut branch) in branches {
                let mut chance = format!("percent_chance {}", chance.value);
                while chance.len() < longest {
                    chance.push(' ');
                }
                self.text(&chance);
                if branch.len() > 0 {
                    self.text(" ");
                    input = self.write_atom(branch.remove(0), input);
                }
            }
        } else {
            for (chance, branch) in branches {
                self.text(&format!("percent_chance {}", chance.value));
                self.newline();
                self.indent += 1;
                for atom in branch {
                    input = self.write_atom(atom, input);
                }
                self.indent -= 1;
            }
        }

        self.widths.pop();

        self.indent -= 1;
        self.text("end_random");
        self.newline();

        input
    }

    /// Write a comment. Multiline comments are formatted Java-style, with a * at the start of each
    /// line.
    fn comment(&mut self, content: &str) {
        self.text("/* ");
        let mut lines = content.lines();
        if let Some(first_line) = lines.next() {
            self.text(first_line.trim());
        }
        let mut is_multiline = false;
        for line in lines {
            is_multiline = true;
            self.newline();
            self.text(" * ");
            if line.trim().starts_with("* ") {
                self.text(
                    &line
                        .chars()
                        .skip_while(|&c| char::is_whitespace(c))
                        .collect::<String>(),
                );
            } else {
                self.text(line);
            }
        }
        if is_multiline {
            self.newline();
        }
        self.text(" */");
        self.newline();
    }

    /// Write a #define statement.
    fn define(&mut self, name: &Word<'_>) {
        self.text("#define ");
        self.text(name.value);
        self.newline();
    }

    /// Write a #const statement.
    fn const_(&mut self, name: &Word<'_>, value: &Option<Word<'_>>) {
        self.text("#const ");
        self.text(name.value);
        self.text(" ");
        if let Some(value) = value {
            self.text(value.value);
        }
        self.newline();
    }

    fn write_atom<I>(&mut self, atom: Atom<'atom>, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'atom>>,
    {
        use Atom::*;

        match (&self.prev, &atom) {
            // Add an additional newline after each }
            (Some(CloseBlock(_)), _) => self.newline(),
            (Some(Other(_)), Other(_)) => (),
            // Add a newline after a run of `Other` tokens
            (Some(Other(_)), _) => self.newline(),
            _ => (),
        }

        match &atom {
            Section(name) => self.section(name),
            Define(_, name) => self.define(name),
            Const(_, name, value) => self.const_(name, value),
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
            If(_, cond) => {
                input = self.condition(cond, input);
            }
            StartRandom(_) => {
                input = self.random(input);
            }
            Comment(_, content, _) => self.comment(content),
            // sometimes people use `//` comments even though that doesn't work
            // should just pass those through
            Other(word) if word.value.starts_with("//") => {
                self.text(word.value);
            }
            Other(word) => {
                let arg_like = word.value.to_ascii_uppercase().as_str() == word.value || word.value.chars().all(|c| c.is_ascii_digit());
                if let (true, Some(Other(_))) = (arg_like, &self.prev) {
                    self.result.push(' ');
                    self.text(word.value);
                } else {
                    self.text(word.value);
                }
            },

            // Garbage non-matching branch statements
            ElseIf(_, cond) => {
                self.text("elseif ");
                self.text(cond.value);
                self.newline();
            },
            Else(_) => {
                self.text("else");
                self.newline();
            },
            EndIf(_) => {
                self.text("endif");
                self.newline();
            },
            CloseBlock(_) => {
                self.text("}");
                self.newline();
            },
            PercentChance(_, chance) => {
                self.text("percent_chance ");
                self.text(chance.value);
                self.newline();
            },
            EndRandom(_) => {
                self.text("end_random");
                self.newline();
            },
        }
        self.prev = Some(atom);
        input
    }

    /// Format a script. Takes an iterator over atoms.
    pub fn format(mut self, input: impl Iterator<Item = Atom<'atom>>) -> String {
        let mut input = input.peekable();
        while let Some(atom) = input.next() {
            input = self.write_atom(atom, input);
        }
        self.result
    }
}

/// Format an rms source string.
pub fn format(source: &str, options: FormatOptions) -> String {
    let mut files = Files::new();
    let f = files.add("format.rms", source);
    let parser = Parser::new(f, files.source(f));
    Formatter::new(options).format(parser.map(|(atom, _)| atom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_section() {
        assert_eq!(
            format("<PLAYER_SETUP> <OBJECTS_GENERATION>", FormatOptions::default()),
            "<PLAYER_SETUP>\r\n\r\n<OBJECTS_GENERATION>\r\n"
        );
    }

    #[test]
    fn command_group() {
        assert_eq!(
            format("create_terrain GRASS3 { base_terrain DESERT border_fuzziness 5 }", FormatOptions::default()),
            "create_terrain GRASS3 {\r\n  base_terrain     DESERT\r\n  border_fuzziness 5\r\n}\r\n"
        );
    }
}
