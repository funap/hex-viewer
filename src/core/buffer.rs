/// A buffer to hold the contents of a file.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Buffer {
    data: Vec<u8>,
}

#[allow(dead_code)]
impl Buffer {
    /// Creates a new Buffer with the given data.
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Creates an empty Buffer.
    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a slice of the buffer's data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns a slice of the buffer's data in the given range.
    /// If the range is out of bounds, it is clamped to the valid range.
    pub fn get_range(&self, start: usize, len: usize) -> &[u8] {
        let start = start.min(self.data.len());
        let end = (start + len).min(self.data.len());
        &self.data[start..end]
    }

    /// Inserts a byte at the specified index.
    pub fn insert(&mut self, index: usize, byte: u8) {
        if index <= self.data.len() {
            self.data.insert(index, byte);
        }
    }

    /// Removes a byte at the specified index and returns it.
    pub fn remove(&mut self, index: usize) -> Option<u8> {
        if index < self.data.len() { Some(self.data.remove(index)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let data = vec![1, 2, 3];
        let buffer = Buffer::new(data.clone());

        assert_eq!(buffer.data(), &data[..]);
        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_empty() {
        let buffer = Buffer::empty();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_get_range() {
        let data = vec![10, 20, 30, 40, 50];
        let buffer = Buffer::new(data);

        // Valid range
        assert_eq!(buffer.get_range(1, 3), &[20, 30, 40]);

        // Range extending beyond end
        assert_eq!(buffer.get_range(3, 10), &[40, 50]);

        // Start beyond end
        assert_eq!(buffer.get_range(10, 5), &[] as &[u8]);

        // Empty range
        assert_eq!(buffer.get_range(0, 0), &[] as &[u8]);
    }
}
