//! A code formatter for AoE2 random map scripts.

use crate::diagnostic::FileId;
use crate::parser::{Atom, AtomKind, Parser};
use crate::tokenizer::Word;
use itertools::Itertools;
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
    fn default() -> Self {
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
    pub const fn tab_size(self, tab_size: u32) -> Self {
        Self { tab_size, ..self }
    }

    /// Whether to use spaces instead of tabs for indentation (default true).
    pub const fn use_spaces(self, use_spaces: bool) -> Self {
        Self { use_spaces, ..self }
    }

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
    pub const fn align_arguments(self, align_arguments: bool) -> Self {
        Self {
            align_arguments,
            ..self
        }
    }

    pub fn format(self, code: &str) -> String {
        let file_id = FileId::new(0);
        let script = Parser::new(file_id, code).map(|(atom, _errors)| atom);
        Formatter::new(self, code).format(script)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Formatter<'file> {
    options: FormatOptions,
    source: &'file str,
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
    prev: Option<Atom<'file>>,
}

impl<'file> Formatter<'file> {
    fn new(options: FormatOptions, source: &'file str) -> Self {
        Self {
            options,
            source,
            ..Default::default()
        }
    }

    /// Get the Kind of the most recently printed Atom, if any exist.
    fn prev_kind(&self) -> Option<&AtomKind<'_>> {
        self.prev.as_ref().map(|atom| &atom.kind)
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
    fn command<'w>(&mut self, name: &Word<'w>, arguments: &[Word<'w>], is_block: bool) {
        self.text(name.value);
        let Width {
            command_width,
            arg_width,
        } = self.widths.last().cloned().unwrap_or_default();

        let mut arg_iter = arguments.iter().peekable();

        if self.options.align_arguments {
            // If we have any arguments, add padding spaces between the command name and arg1, and between
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
        if self.prev.is_some() {
            self.newline();
        }
        self.text(name.value);
        self.newline();
    }

    /// Write a command block. This reads atoms from the iterator until the end of the block, and
    /// writes both the command and any attributes it may contain.
    fn block<I>(&mut self, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'file>>,
    {
        let is_end = |atom: &Atom<'_>| matches!(atom.kind, AtomKind::CloseBlock { .. });

        self.inside_block += 1;

        let mut commands = vec![];
        let mut width = Width::default();
        let mut indent = 0;
        for atom in input.by_ref().take_while(|atom| !is_end(atom)) {
            width = match &atom.kind {
                AtomKind::Command { name, arguments } => Width {
                    command_width: width
                        .command_width
                        .max(name.value.len() + indent * self.options.tab_size as usize),
                    arg_width: width
                        .arg_width
                        .max(arguments.get(0).map(|word| word.value.len()).unwrap_or(0)),
                },
                AtomKind::If { .. } => {
                    indent += 1;
                    width
                }
                AtomKind::EndIf { .. } => {
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

        // Manually add newline if there was garbage
        if let Some(AtomKind::Other { .. }) = self.prev_kind() {
            self.newline();
        }

        self.inside_block -= 1;
        self.indent -= 1;
        self.text("}");
        self.newline();

        input
    }

    fn condition<I>(&mut self, cond: &Word<'_>, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'file>>,
    {
        self.text("if ");
        self.text(cond.value);
        self.newline();
        self.indent += 1;

        // reset command width so an if block within a command block
        // does not over-indent.
        let Width {
            command_width,
            arg_width,
        } = self.widths.last().cloned().unwrap_or_default();
        self.widths.push(Width {
            command_width: command_width.saturating_sub(2),
            arg_width,
        });

        let mut depth = 1;
        let mut sub_input = input
            .by_ref()
            .peeking_take_while(|atom| {
                match atom.kind {
                    AtomKind::If { .. } => depth += 1,
                    AtomKind::EndIf { .. } => depth -= 1,
                    _ => (),
                }

                // end on an endif atom at depth 0, else continue
                depth > 0 || !matches!(atom.kind, AtomKind::EndIf { .. })
            })
            .peekable();

        while let Some(atom) = sub_input.next() {
            match &atom.kind {
                AtomKind::ElseIf { .. } | AtomKind::Else { .. } => {
                    self.indent -= 1;
                    sub_input = self.write_atom(atom, sub_input);
                    self.indent += 1;
                }
                _ => {
                    sub_input = self.write_atom(atom, sub_input);
                }
            }
        }

        self.widths.pop();

        self.indent -= 1;
        let endif = input.next().unwrap();
        let mut input = self.write_atom(endif, input);

        if self.inside_block == 0 {
            let next_kind = input.peek().map(|atom| &atom.kind);
            if let Some(AtomKind::OpenBlock { .. }) = next_kind {
                // No newline before an open brace:
                // ```
                // if X
                //   create_object Y
                // endif
                // {
                // ```
            } else if next_kind.is_some() {
                // TODO maybe get rid of this entirely?
                self.newline();
            }
        }

        input
    }

    fn random<I>(&mut self, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'file>>,
    {
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
            match atom.kind {
                AtomKind::PercentChance { chance, .. } if depth == 1 => {
                    branches.push((chance, vec![]));
                    continue;
                }
                AtomKind::StartRandom { .. } => {
                    depth += 1;
                }
                AtomKind::EndRandom { .. } => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => (),
            };

            if branches.is_empty() {
                null_branch.push(atom);
            } else {
                branches.last_mut().unwrap().1.push(atom);
            }
        }

        let has_simple_branches = branches.iter().all(|(_, stmts)| {
            if stmts.len() > 1 {
                return false;
            }
            if stmts.is_empty() {
                return true;
            }
            matches!(
                stmts[0].kind,
                AtomKind::Define { .. }
                    | AtomKind::Const { .. }
                    | AtomKind::Undefine { .. }
                    | AtomKind::Command { .. }
            )
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
                if !branch.is_empty() {
                    self.text(" ");
                    input = self.write_atom(branch.remove(0), input);
                } else {
                    self.newline();
                }
            }
        } else {
            for (chance, branch) in branches {
                self.text(&format!("percent_chance {}", chance.value));
                self.newline();
                self.indent += 1;

                let mut sub_input = branch.into_iter().peekable();
                while let Some(atom) = sub_input.next() {
                    sub_input = self.write_atom(atom, sub_input);
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

    /// Write an #undefine statement.
    fn undefine(&mut self, name: &Word<'_>) {
        self.text("#undefine ");
        self.text(name.value);
        self.newline();
    }

    /// Is there a padding line between the atoms `prev` and `next`?
    ///
    /// A padding line is defined as a newline, followed by whitespace, followed by another newline.
    fn has_padding_line(&self, prev: &Atom<'_>, next: &Atom<'_>) -> bool {
        let input = &self.source[prev.location.end().into()..next.location.start().into()];
        let empty_lines = input.lines().filter(|line| line.trim().is_empty());
        // at least 2 subsequent newlines? i.e., at least 3 lines?
        // If input ends with a newline, `.lines()` doesn't generate a final empty item, so it will
        // only output two items.
        if input.ends_with('\n') {
            empty_lines.take(2).count() >= 2
        } else {
            empty_lines.take(3).count() >= 3
        }
    }

    /// Should the `next` atom be written at the end of the line `prev` is on?
    ///
    /// If the `next` atom is a comment, and the input did not put a newline between the `prev` and
    /// `next` atoms, it should.
    fn should_comment_be_on_same_line(&self, prev: &Atom<'_>, next: &Atom<'_>) -> bool {
        let input = &self.source[prev.location.end().into()..next.location.start().into()];
        if let AtomKind::Comment { .. } = &next.kind {
            !input.contains('\n')
        } else {
            false
        }
    }

    fn write_atom<I>(&mut self, atom: Atom<'file>, mut input: Peekable<I>) -> Peekable<I>
    where
        I: Iterator<Item = Atom<'file>>,
    {
        match (self.prev_kind(), &atom.kind) {
            // Add an additional newline after each }
            (Some(AtomKind::CloseBlock { .. }), _) => self.newline(),
            (Some(AtomKind::Other { .. }), AtomKind::Other { .. }) => (),
            // Add a newline after a run of `Other` tokens
            (Some(AtomKind::Other { .. }), _) => self.newline(),
            _ => (),
        }

        if let Some(prev) = &self.prev {
            // special whitespace handling:
            // - Maintain padding lines.
            // - Do not add linebreak before comments at the end of a line

            if self.has_padding_line(prev, &atom) {
                // A padding line may already have been added by the formatter for another reason,
                // like after top-level `endif`s. Don't add another in that case.
                if !self.result.ends_with("\r\n\r\n") {
                    self.newline();
                }
            } else if self.should_comment_be_on_same_line(prev, &atom) {
                if self.result.ends_with("\r\n") {
                    self.result.pop();
                    self.result.pop();
                    self.needs_indent = false;
                }
                self.text(" ");
            }
        }

        match &atom.kind {
            AtomKind::Section { name, .. } => self.section(name),
            AtomKind::Define { name, .. } => self.define(name),
            AtomKind::Const { name, value, .. } => self.const_(name, value),
            AtomKind::Undefine { name, .. } => self.undefine(name),
            AtomKind::Command { name, arguments } => {
                let is_block = matches!(
                    input.peek().map(|atom| &atom.kind),
                    Some(AtomKind::OpenBlock { .. })
                );
                self.command(name, arguments, is_block);
            }
            AtomKind::Comment { content, .. } => self.comment(content),
            // sometimes people use `//` comments even though that doesn't work
            // should just pass those through
            AtomKind::Other { value } if value.value.starts_with("//") => {
                self.text(value.value);
            }
            AtomKind::Other { value } => {
                let arg_like = value.value.to_ascii_uppercase().as_str() == value.value
                    || value.value.chars().all(|c| c.is_ascii_digit());
                if let (true, Some(AtomKind::Other { .. })) = (arg_like, self.prev_kind()) {
                    self.result.push(' ');
                    self.text(value.value);
                } else {
                    self.text(value.value);
                }
            }

            // Chunks of other control flow constructs. When encountering the start of one of these
            // constructs, the formatter calls a separate method that will deal with the entire
            // construct. They do not need special handling here. These atoms may also appear in
            // incorrect positions, and then we don't want special handling either. So, for these
            // cases we only print the command as is.
            AtomKind::ElseIf { condition, .. } => {
                self.text("elseif ");
                self.text(condition.value);
                self.newline();
            }
            AtomKind::Else { .. } => {
                self.text("else");
                self.newline();
            }
            AtomKind::EndIf { .. } => {
                self.text("endif");
                self.newline();
            }
            AtomKind::CloseBlock { .. } => {
                self.text("}");
                self.newline();
            }
            AtomKind::PercentChance { chance, .. } => {
                self.text("percent_chance ");
                self.text(chance.value);
                self.newline();
            }
            AtomKind::EndRandom { .. } => {
                self.text("end_random");
                self.newline();
            }

            // These call into methods that do nested `write_atom` calls. They need to update
            // `prev` first and not do anything _after_ calling the method to avoid getting in a
            // bad state.
            //
            // FIXME It would be nice to approach this differently!
            AtomKind::OpenBlock { .. } => {
                self.prev = Some(atom);
                return self.block(input);
            }
            AtomKind::If { condition, .. } => {
                let condition = *condition;
                self.prev = Some(atom);
                return self.condition(&condition, input);
            }
            AtomKind::StartRandom { .. } => {
                self.prev = Some(atom);
                return self.random(input);
            }
        }
        self.prev = Some(atom);
        input
    }

    /// Format a script. Takes an iterator over atoms.
    pub fn format(mut self, input: impl Iterator<Item = Atom<'file>>) -> String {
        let mut input = input.peekable();
        while let Some(atom) = input.next() {
            input = self.write_atom(atom, input);
        }
        self.result
    }
}

/// Format an rms source string.
pub fn format(source: &str, options: FormatOptions) -> String {
    options.format(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_section() {
        assert_eq!(
            format(
                "<PLAYER_SETUP> <OBJECTS_GENERATION>",
                FormatOptions::default()
            ),
            "<PLAYER_SETUP>\r\n\r\n<OBJECTS_GENERATION>\r\n"
        );
    }

    #[test]
    fn command_group() {
        assert_eq!(
            format(
                "create_terrain GRASS3 { base_terrain DESERT border_fuzziness 5 }",
                FormatOptions::default()
            ),
            "create_terrain GRASS3 {\r\n  base_terrain     DESERT\r\n  border_fuzziness 5\r\n}\r\n"
        );
    }

    #[test]
    fn retain_whitespace() {
        assert_eq!(
            format(
                "create_terrain GRASS3 {\r\n\r\nbase_terrain DESERT\r\n\r\nborder_fuzziness 5 }",
                FormatOptions::default()
            ),
            "create_terrain GRASS3 {\r\n\r\n  base_terrain     DESERT\r\n\r\n  border_fuzziness 5\r\n}\r\n"
        );
    }

    /// This one fails with one too many newline
    #[ignore]
    #[test]
    fn retain_whitespace_comment() {
        assert_eq!(
            format("if A /* comment */ endif\r\n", FormatOptions::default()),
            "if A /* comment */\r\nendif\r\n"
        );
        assert_eq!(
            format(
                "#define A\r\n\r\n/* *** *** */\r\n\r\n<PLAYER_SETUP>",
                FormatOptions::default()
            ),
            "#define A\r\n\r\n/* *** *** */\r\n\r\n<PLAYER_SETUP>\r\n"
        );
    }

    #[test]
    fn retain_whitespace_if() {
        assert_eq!(
            format("if A #define X else endif", FormatOptions::default()),
            "if A\r\n  #define X\r\nelse\r\nendif\r\n"
        );
        assert_eq!(
            format(
                "if A\n\n#define X\n\n\n\nelse\n\n\n\n\n\n\nendif",
                FormatOptions::default()
            ),
            "if A\r\n\r\n  #define X\r\n\r\nelse\r\n\r\nendif\r\n"
        );
    }
}
