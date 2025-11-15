pub struct ZBuffer {
    buffer: Vec<f32>,
    width: usize,
    height: usize,
}

impl ZBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        ZBuffer {
            buffer: vec![f32::INFINITY; width * height],
            width,
            height,
        }
    }
    
    pub fn clear(&mut self) {
        self.buffer.fill(f32::INFINITY);
    }
    
    pub fn test_and_set(&mut self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        let idx = (y as usize) * self.width + (x as usize);
        if depth < self.buffer[idx] {
            self.buffer[idx] = depth;
            true
        } else {
            false
        }
    }
}
