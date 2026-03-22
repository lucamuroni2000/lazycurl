/// A single-line text input with cursor support.
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    /// The current text content
    content: String,
    /// Cursor position (byte index)
    cursor: usize,
}

impl TextInput {
    pub fn new(initial: &str) -> Self {
        let len = initial.len();
        Self {
            content: initial.to_string(),
            cursor: len,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, s: &str) {
        self.content = s.to_string();
        self.cursor = self.content.len();
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor > 0 {
            // Find the previous char boundary
            let prev = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += self.content[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let input = TextInput::default();
        assert_eq!(input.content(), "");
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn test_new_with_content() {
        let input = TextInput::new("hello");
        assert_eq!(input.content(), "hello");
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_insert_char() {
        let mut input = TextInput::default();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_insert_in_middle() {
        let mut input = TextInput::new("hllo");
        input.cursor = 1;
        input.insert_char('e');
        assert_eq!(input.content(), "hello");
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_delete_char_before() {
        let mut input = TextInput::new("hello");
        input.delete_char_before();
        assert_eq!(input.content(), "hell");
        assert_eq!(input.cursor(), 4);
    }

    #[test]
    fn test_delete_char_before_at_start() {
        let mut input = TextInput::new("hello");
        input.cursor = 0;
        input.delete_char_before();
        assert_eq!(input.content(), "hello");
    }

    #[test]
    fn test_delete_char_after() {
        let mut input = TextInput::new("hello");
        input.cursor = 0;
        input.delete_char_after();
        assert_eq!(input.content(), "ello");
    }

    #[test]
    fn test_move_left_right() {
        let mut input = TextInput::new("abc");
        assert_eq!(input.cursor(), 3);
        input.move_left();
        assert_eq!(input.cursor(), 2);
        input.move_left();
        assert_eq!(input.cursor(), 1);
        input.move_right();
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn test_move_home_end() {
        let mut input = TextInput::new("hello");
        input.move_home();
        assert_eq!(input.cursor(), 0);
        input.move_end();
        assert_eq!(input.cursor(), 5);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("hello");
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor(), 0);
    }

    #[test]
    fn test_set_content() {
        let mut input = TextInput::default();
        input.set_content("new value");
        assert_eq!(input.content(), "new value");
        assert_eq!(input.cursor(), 9);
    }
}
