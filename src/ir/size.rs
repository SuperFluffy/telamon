use ir;
use num;
use std;
use std::borrow::Borrow;

/// The size of an iteration dimension. The size is of the form:
/// `(factor * dividend_0 * dividend_1 * ...)) / divisor`
/// where the reminder of the division is null.
#[derive(Clone, Debug)]
pub struct Size<'a> {
    factor: u32,
    dividend: Vec<&'a ir::Parameter>,
    divisor: u32,
    universe: Vec<u32>,
}

impl<'a> Size<'a> {
    /// Creates a new 'Size'.
    pub fn new(factor: u32, dividend: Vec<&'a ir::Parameter>, divisor: u32) -> Self {
        assert!(factor != 0);
        assert!(divisor != 0);
        let universe = if dividend.is_empty() { vec![factor] } else { vec![] };
        let mut new = Size { factor, dividend, divisor, universe };
        new.simplify();
        new
    }

    /// Returns the values the size can take, if the size is statically known.
    pub fn universe(&self) -> Option<&[u32]> {
        if self.dividend.is_empty() { Some(&self.universe) } else { None }
    }

    /// Returns the size of a dimension if it is staticaly known and doesn't depend on
    /// any choice.
    pub fn as_fixed(&self) -> Option<u32> {
        if self.dividend.is_empty() { Some(self.factor) } else { None }
    }

    /// Indicates if the size is constant.
    pub fn is_constant(&self) -> bool { self.dividend.is_empty() }

    /// Returns the dividends.
    pub fn dividend(&self) -> &[&'a ir::Parameter] { &self.dividend }

    /// Returns the divisor.
    pub fn divisor(&self) -> u32 { self.divisor }

    /// Returns the factor.
    pub fn factor(&self) -> u32 { self.factor }

    /// Multiplies the divisor by the given factor.
    pub fn mul_divisor(&mut self, d: u32) {
        assert_ne!(d, 0);
        self.divisor *= d;
        self.simplify();
    }

    /// Multiplies the factor by the given factor.
    pub fn mul_factor(&mut self, d: u32) {
        assert_ne!(d, 0);
        self.factor *= d;
        self.simplify();
    }

    /// Simplifies the fraction factor/divisor.
    fn simplify(&mut self) {
        let gcd = num::integer::gcd(self.factor, self.divisor);
        self.factor /= gcd;
        self.divisor /= gcd;
        if self.dividend.is_empty() { self.universe[0] = self.factor; }
    }

    /// Indicates if the dimension can be merged with another, assuming they have the
    /// same tiling factor and size choice value.
    pub fn is_compatible_with(&self, other: &ir::Size) -> bool {
        self.factor == other.factor && self.dividend == other.dividend
            && self.divisor == other.divisor
    }
}

impl<'a, T> std::ops::MulAssign<T> for Size<'a> where T: Borrow<ir::Size<'a>> {
    fn mul_assign(&mut self, rhs: T) {
        self.factor *= rhs.borrow().factor;
        self.dividend.extend(rhs.borrow().dividend.iter().cloned());
        self.divisor *= rhs.borrow().divisor;
        self.simplify();
    }
}

impl<'a, T> std::ops::Mul<T> for Size<'a> where T: Borrow<ir::Size<'a>> {
    type Output = Size<'a>;

    fn mul(mut self, rhs: T) -> Self::Output {
        self *= rhs;
        self
    }
}

impl<'a, 'b> std::iter::Product<&'b ir::Size<'a>> for ir::Size<'a> where 'a: 'b  {
    fn product<I>(iter: I) -> Self where I: Iterator<Item=&'b ir::Size<'a>> {
        let mut factor = 1;
        let mut dividend = vec![];
        let mut divisor = 1;
        for s in iter {
            factor *= s.factor;
            dividend.extend(s.dividend.iter().cloned());
            divisor *= s.divisor;
        }
        ir::Size::new(factor, dividend, divisor)
    }
}
