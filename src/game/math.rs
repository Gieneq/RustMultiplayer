use serde::{
    Deserialize, 
    Serialize
};

pub type Vector2F = Vector2X<f32>;
pub type Vector2U = Vector2X<u32>;
pub type Vector2I = Vector2X<i32>;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Vector2X<T> {
    pub x: T,
    pub y: T,
}

pub type Rect2F = Rect2X<f32>;
pub type Rect2U = Rect2X<u32>;
pub type Rect2I = Rect2X<i32>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rect2X<T> {
    pub pos: Vector2X<T>,
    pub size: Vector2X<T>,
}

impl<T: std::fmt::Display> std::fmt::Display for Vector2X<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Rect2X<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[({},{}), ({},{})]", self.pos.x, self.pos.y, self.size.x, self.size.y)
    }
}

impl<T> Vector2X<T> 
where 
    T: Default
{
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: T::default(), y: T::default() }
    }
}

impl<T> Vector2X<T> 
where 
    T: Into<f32> + Copy
{
    pub fn length_squared(&self) -> f32 {
        let xf: f32 = T::into(self.x); 
        let yf: f32 = T::into(self.y); 
        xf.powi(2) + yf.powi(2)
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normal(&self) -> Vector2X<f32> {
        let len = self.length();
        Vector2X {
            x: T::into(self.x) / len,
            y: T::into(self.y) / len,
        }
    }
}

impl Vector2X<f32>
{
    pub fn dot(&self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl<T> std::ops::Add for Vector2X<T> 
where 
    T: std::ops::Add<Output = T>
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x, 
            y: self.y + rhs.y
        }
        
    }
}

impl<T> std::ops::AddAssign for Vector2X<T> 
where
    T: std::ops::AddAssign
{
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl<T> std::ops::Neg for Vector2X<T> 
where 
    T: std::ops::Neg<Output = T>
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: T::neg(self.x),
            y: T::neg(self.y),
        }
    }
}

impl<T> std::ops::Mul<T> for Vector2X<T> 
where 
    T: std::ops::Mul<Output = T> + Copy
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs
        }
    }
}

impl<T> std::ops::Sub for Vector2X<T> 
where 
    T: std::ops::Sub<Output = T>
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: T::sub(self.x, rhs.x),
            y: T::sub(self.y, rhs.y)
        }
    }
}

impl From<Vector2X<f32>> for Vector2X<u32> {
    fn from(value: Vector2X<f32>) -> Self {
        Self { x: value.x as u32, y: value.y as u32 }
    }
}

impl From<Vector2X<u32>> for Vector2X<f32> {
    fn from(value: Vector2X<u32>) -> Self {
        Self { x: value.x as f32, y: value.y as f32 }
    }
}

impl<T> Rect2X<T> {
    pub fn new(x: T, y: T, w: T, h: T) -> Self {
        Self { pos: Vector2X { x, y }, size: Vector2X { x: w, y: h } }
    }
}

impl<T> Rect2X<T> 
where 
    T: PartialOrd + std::ops::Add<Output = T> + Copy
{
    pub fn contains(&self, point: &Vector2X<T>) -> bool {
        point.x >= self.pos.x
            && point.y >= self.pos.y
            && point.x < self.pos.x + self.size.x
            && point.y < self.pos.y + self.size.y
    }
}

#[test]
fn test_vector_creation() {
    let v1 = Vector2X::<f32>::new(1.0, 2.0);
    assert_eq!(v1.x, 1.0);
    assert_eq!(v1.y, 2.0);
}

#[test]
fn test_vector_add() {
    let v1 = Vector2X::<u32>::new(1, 2);
    let v2 = Vector2X::<u32>::new(10, 20);
    let v3 = v1 + v2;
    assert_eq!(v3.x, v1.x + v2.x);
    assert_eq!(v3.y, v1.y + v2.y);
}

#[test]
fn test_vector_add_assign() {
    let v1 = Vector2X::<u32>::new(1, 2);
    let mut v2 = Vector2X::<u32>::new(10, 20);
    v2 += v1;
    assert_eq!(v2.x, 11);
    assert_eq!(v2.y, 22);
}

#[test]
fn test_vector_negation() {
    let v1 = Vector2X::<i32>::new(1, 2);
    let v1_neg = -v1;
    assert_eq!(v1_neg.x, -v1.x);
    assert_eq!(v1_neg.y, -v1.y);
}

#[test]
fn test_vector_mul_scalar() {
    let v1 = Vector2X::<i32>::new(1, 2);
    let scalar = 5;
    let v1_multiplied = v1 * scalar;
    assert_eq!(v1_multiplied.x, v1.x * scalar);
    assert_eq!(v1_multiplied.y, v1.y * scalar);
}

#[test]
fn test_vector_casting() {
    let v1 = Vector2X::<f32>::new(1.2, 2.6);
    let v1_cast_u32 =  Vector2X::<u32>::from(v1);
    assert_eq!(v1_cast_u32.x, 1);
    assert_eq!(v1_cast_u32.y, 2);
}

#[test]
fn test_vector_dot() {
    let v1 = Vector2X::<f32>::new(1.0, 0.0);
    let v2 = Vector2X::<f32>::new(-1.0, 0.0);
    let v1_dot_v2 = v1.dot(v2);
    assert_eq!(v1_dot_v2, -1.0);
}

#[test]
fn test_rect_creation() {
    let position = Vector2X::<f32>::new(1.0, 0.0);
    let size = Vector2X::<f32>::new(3.0, 5.0);
    let rect = Rect2X::new(position.x, position.y, size.x, size.y);
    assert_eq!(rect.pos, position);
    assert_eq!(rect.size, size);
}

#[test]
fn test_rect_containing() {
    let position = Vector2X::<f32>::new(1.0, 0.0);
    let size = Vector2X::<f32>::new(3.0, 5.0);
    let rect = Rect2X::new(position.x, position.y, size.x, size.y);

    let p1_inside = position;
    let p2_not_inside = position + Vector2X::new(size.x, 0.0);
    let p3_not_inside = position + Vector2X::new(0.0, size.y);
    let p4_not_inside = position + size;
    let p5_inside = position + Vector2X::new(size.x / 2.0, size.y / 2.0);

    assert!(rect.contains(&p1_inside));
    assert!(!rect.contains(&p2_not_inside));
    assert!(!rect.contains(&p3_not_inside));
    assert!(!rect.contains(&p4_not_inside));
    assert!(rect.contains(&p5_inside));
}