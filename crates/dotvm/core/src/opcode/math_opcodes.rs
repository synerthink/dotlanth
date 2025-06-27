// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Advanced mathematical operation opcodes for DotVM
//!
//! This module defines opcodes for high-precision mathematical operations
//! and advanced mathematical functions available on 256-bit+ architectures.

use std::fmt;

/// Advanced mathematical opcodes for 256-bit+ architectures
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum MathOpcode {
    // High-precision arithmetic (256-bit+)
    HighPrecisionAdd = 0x01,     // High-precision addition
    HighPrecisionSub = 0x02,     // High-precision subtraction
    HighPrecisionMul = 0x03,     // High-precision multiplication
    HighPrecisionDiv = 0x04,     // High-precision division
    HighPrecisionMod = 0x05,     // High-precision modulo
    
    // High-precision elementary functions
    HighPrecisionSqrt = 0x10,    // High-precision square root
    HighPrecisionPow = 0x11,     // High-precision power
    HighPrecisionExp = 0x12,     // High-precision exponential
    HighPrecisionLog = 0x13,     // High-precision natural logarithm
    HighPrecisionLog10 = 0x14,   // High-precision base-10 logarithm
    HighPrecisionLog2 = 0x15,    // High-precision base-2 logarithm
    
    // High-precision trigonometric functions
    HighPrecisionSin = 0x20,     // High-precision sine
    HighPrecisionCos = 0x21,     // High-precision cosine
    HighPrecisionTan = 0x22,     // High-precision tangent
    HighPrecisionAsin = 0x23,    // High-precision arcsine
    HighPrecisionAcos = 0x24,    // High-precision arccosine
    HighPrecisionAtan = 0x25,    // High-precision arctangent
    HighPrecisionAtan2 = 0x26,   // High-precision two-argument arctangent
    
    // High-precision hyperbolic functions
    HighPrecisionSinh = 0x30,    // High-precision hyperbolic sine
    HighPrecisionCosh = 0x31,    // High-precision hyperbolic cosine
    HighPrecisionTanh = 0x32,    // High-precision hyperbolic tangent
    HighPrecisionAsinh = 0x33,   // High-precision inverse hyperbolic sine
    HighPrecisionAcosh = 0x34,   // High-precision inverse hyperbolic cosine
    HighPrecisionAtanh = 0x35,   // High-precision inverse hyperbolic tangent
    
    // Special mathematical functions
    Gamma = 0x40,                // Gamma function
    LogGamma = 0x41,             // Log gamma function
    Beta = 0x42,                 // Beta function
    Erf = 0x43,                  // Error function
    Erfc = 0x44,                 // Complementary error function
    Bessel = 0x45,               // Bessel functions
    
    // Number theory functions
    ModularExp = 0x50,           // Modular exponentiation
    DiscreteLog = 0x51,          // Discrete logarithm
    Jacobi = 0x52,               // Jacobi symbol
    Legendre = 0x53,             // Legendre symbol
    
    // Statistical functions
    Mean = 0x60,                 // Arithmetic mean
    GeometricMean = 0x61,        // Geometric mean
    HarmonicMean = 0x62,         // Harmonic mean
    Variance = 0x63,             // Variance
    StandardDeviation = 0x64,    // Standard deviation
    Skewness = 0x65,             // Skewness
    Kurtosis = 0x66,             // Kurtosis
    Correlation = 0x67,          // Correlation coefficient
    Covariance = 0x68,           // Covariance
    
    // Linear algebra functions
    Determinant = 0x70,          // Matrix determinant
    Trace = 0x71,                // Matrix trace
    Rank = 0x72,                 // Matrix rank
    Eigenvalues = 0x73,          // Matrix eigenvalues
    Eigenvectors = 0x74,         // Matrix eigenvectors
    SingularValues = 0x75,       // Singular value decomposition
    
    // Optimization functions
    Minimize = 0x80,             // Function minimization
    Maximize = 0x81,             // Function maximization
    FindRoot = 0x82,             // Root finding
    Integrate = 0x83,            // Numerical integration
    Differentiate = 0x84,        // Numerical differentiation
    
    // Precision control
    SetPrecision = 0x90,         // Set working precision
    GetPrecision = 0x91,         // Get current precision
    RoundToNearest = 0x92,       // Round to nearest
    RoundUp = 0x93,              // Round up (ceiling)
    RoundDown = 0x94,            // Round down (floor)
    Truncate = 0x95,             // Truncate
}

impl MathOpcode {
    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> usize {
        match self {
            // Unary operations
            MathOpcode::HighPrecisionSqrt |
            MathOpcode::HighPrecisionExp |
            MathOpcode::HighPrecisionLog |
            MathOpcode::HighPrecisionLog10 |
            MathOpcode::HighPrecisionLog2 |
            MathOpcode::HighPrecisionSin |
            MathOpcode::HighPrecisionCos |
            MathOpcode::HighPrecisionTan |
            MathOpcode::HighPrecisionAsin |
            MathOpcode::HighPrecisionAcos |
            MathOpcode::HighPrecisionAtan |
            MathOpcode::HighPrecisionSinh |
            MathOpcode::HighPrecisionCosh |
            MathOpcode::HighPrecisionTanh |
            MathOpcode::HighPrecisionAsinh |
            MathOpcode::HighPrecisionAcosh |
            MathOpcode::HighPrecisionAtanh |
            MathOpcode::Gamma |
            MathOpcode::LogGamma |
            MathOpcode::Erf |
            MathOpcode::Erfc |
            MathOpcode::Determinant |
            MathOpcode::Trace |
            MathOpcode::Rank |
            MathOpcode::Eigenvalues |
            MathOpcode::Eigenvectors |
            MathOpcode::SingularValues |
            MathOpcode::GetPrecision |
            MathOpcode::RoundToNearest |
            MathOpcode::RoundUp |
            MathOpcode::RoundDown |
            MathOpcode::Truncate => 1,
            
            // Binary operations
            MathOpcode::HighPrecisionAdd |
            MathOpcode::HighPrecisionSub |
            MathOpcode::HighPrecisionMul |
            MathOpcode::HighPrecisionDiv |
            MathOpcode::HighPrecisionMod |
            MathOpcode::HighPrecisionPow |
            MathOpcode::HighPrecisionAtan2 |
            MathOpcode::Beta |
            MathOpcode::Bessel |
            MathOpcode::DiscreteLog |
            MathOpcode::Jacobi |
            MathOpcode::Legendre |
            MathOpcode::Correlation |
            MathOpcode::Covariance |
            MathOpcode::SetPrecision => 2,
            
            // Ternary operations
            MathOpcode::ModularExp => 3,
            
            // Variable operand operations (array-based)
            MathOpcode::Mean |
            MathOpcode::GeometricMean |
            MathOpcode::HarmonicMean |
            MathOpcode::Variance |
            MathOpcode::StandardDeviation |
            MathOpcode::Skewness |
            MathOpcode::Kurtosis => 1, // Takes array as single operand
            
            // Complex operations
            MathOpcode::Minimize |
            MathOpcode::Maximize |
            MathOpcode::FindRoot |
            MathOpcode::Integrate |
            MathOpcode::Differentiate => 2, // Function + parameters
        }
    }

    /// Get the minimum precision required for this operation
    pub fn minimum_precision(&self) -> u32 {
        match self {
            // Standard precision operations
            MathOpcode::HighPrecisionAdd |
            MathOpcode::HighPrecisionSub |
            MathOpcode::HighPrecisionMul |
            MathOpcode::HighPrecisionDiv => 128,
            
            // High precision operations
            MathOpcode::HighPrecisionSqrt |
            MathOpcode::HighPrecisionPow |
            MathOpcode::HighPrecisionExp |
            MathOpcode::HighPrecisionLog |
            MathOpcode::HighPrecisionLog10 |
            MathOpcode::HighPrecisionLog2 => 256,
            
            // Very high precision operations
            MathOpcode::HighPrecisionSin |
            MathOpcode::HighPrecisionCos |
            MathOpcode::HighPrecisionTan |
            MathOpcode::HighPrecisionAsin |
            MathOpcode::HighPrecisionAcos |
            MathOpcode::HighPrecisionAtan |
            MathOpcode::HighPrecisionAtan2 |
            MathOpcode::HighPrecisionSinh |
            MathOpcode::HighPrecisionCosh |
            MathOpcode::HighPrecisionTanh |
            MathOpcode::HighPrecisionAsinh |
            MathOpcode::HighPrecisionAcosh |
            MathOpcode::HighPrecisionAtanh => 512,
            
            // Special functions requiring ultra-high precision
            MathOpcode::Gamma |
            MathOpcode::LogGamma |
            MathOpcode::Beta |
            MathOpcode::Erf |
            MathOpcode::Erfc |
            MathOpcode::Bessel => 1024,
            
            // Other operations
            _ => 128,
        }
    }

    /// Check if this operation is commutative
    pub fn is_commutative(&self) -> bool {
        matches!(self,
            MathOpcode::HighPrecisionAdd |
            MathOpcode::HighPrecisionMul |
            MathOpcode::Beta |
            MathOpcode::Correlation |
            MathOpcode::Covariance
        )
    }

    /// Get the computational complexity category
    pub fn complexity(&self) -> ComputationalComplexity {
        match self {
            // Linear operations
            MathOpcode::HighPrecisionAdd |
            MathOpcode::HighPrecisionSub |
            MathOpcode::Mean |
            MathOpcode::Variance |
            MathOpcode::StandardDeviation => ComputationalComplexity::Linear,
            
            // Quadratic operations
            MathOpcode::HighPrecisionMul |
            MathOpcode::HighPrecisionDiv |
            MathOpcode::Correlation |
            MathOpcode::Covariance => ComputationalComplexity::Quadratic,
            
            // Cubic operations
            MathOpcode::HighPrecisionPow |
            MathOpcode::ModularExp |
            MathOpcode::Determinant => ComputationalComplexity::Cubic,
            
            // Logarithmic operations
            MathOpcode::HighPrecisionSqrt |
            MathOpcode::HighPrecisionLog |
            MathOpcode::HighPrecisionLog10 |
            MathOpcode::HighPrecisionLog2 => ComputationalComplexity::Logarithmic,
            
            // Exponential operations
            MathOpcode::HighPrecisionExp |
            MathOpcode::HighPrecisionSin |
            MathOpcode::HighPrecisionCos |
            MathOpcode::HighPrecisionTan |
            MathOpcode::Gamma |
            MathOpcode::Bessel => ComputationalComplexity::Exponential,
            
            // Very complex operations
            MathOpcode::Eigenvalues |
            MathOpcode::Eigenvectors |
            MathOpcode::SingularValues |
            MathOpcode::Minimize |
            MathOpcode::Maximize |
            MathOpcode::FindRoot |
            MathOpcode::Integrate |
            MathOpcode::Differentiate => ComputationalComplexity::SuperExponential,
            
            // Simple operations
            _ => ComputationalComplexity::Constant,
        }
    }
}

/// Computational complexity categories for mathematical operations
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComputationalComplexity {
    Constant,         // O(1)
    Logarithmic,      // O(log n)
    Linear,           // O(n)
    Quadratic,        // O(n²)
    Cubic,            // O(n³)
    Exponential,      // O(2^n)
    SuperExponential, // O(n!)
}

impl fmt::Display for MathOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MathOpcode::HighPrecisionAdd => write!(f, "hp.add"),
            MathOpcode::HighPrecisionSub => write!(f, "hp.sub"),
            MathOpcode::HighPrecisionMul => write!(f, "hp.mul"),
            MathOpcode::HighPrecisionDiv => write!(f, "hp.div"),
            MathOpcode::HighPrecisionMod => write!(f, "hp.mod"),
            MathOpcode::HighPrecisionSqrt => write!(f, "hp.sqrt"),
            MathOpcode::HighPrecisionPow => write!(f, "hp.pow"),
            MathOpcode::HighPrecisionExp => write!(f, "hp.exp"),
            MathOpcode::HighPrecisionLog => write!(f, "hp.log"),
            MathOpcode::HighPrecisionLog10 => write!(f, "hp.log10"),
            MathOpcode::HighPrecisionLog2 => write!(f, "hp.log2"),
            MathOpcode::HighPrecisionSin => write!(f, "hp.sin"),
            MathOpcode::HighPrecisionCos => write!(f, "hp.cos"),
            MathOpcode::HighPrecisionTan => write!(f, "hp.tan"),
            MathOpcode::HighPrecisionAsin => write!(f, "hp.asin"),
            MathOpcode::HighPrecisionAcos => write!(f, "hp.acos"),
            MathOpcode::HighPrecisionAtan => write!(f, "hp.atan"),
            MathOpcode::HighPrecisionAtan2 => write!(f, "hp.atan2"),
            MathOpcode::HighPrecisionSinh => write!(f, "hp.sinh"),
            MathOpcode::HighPrecisionCosh => write!(f, "hp.cosh"),
            MathOpcode::HighPrecisionTanh => write!(f, "hp.tanh"),
            MathOpcode::HighPrecisionAsinh => write!(f, "hp.asinh"),
            MathOpcode::HighPrecisionAcosh => write!(f, "hp.acosh"),
            MathOpcode::HighPrecisionAtanh => write!(f, "hp.atanh"),
            MathOpcode::Gamma => write!(f, "math.gamma"),
            MathOpcode::LogGamma => write!(f, "math.loggamma"),
            MathOpcode::Beta => write!(f, "math.beta"),
            MathOpcode::Erf => write!(f, "math.erf"),
            MathOpcode::Erfc => write!(f, "math.erfc"),
            MathOpcode::Bessel => write!(f, "math.bessel"),
            MathOpcode::ModularExp => write!(f, "math.modexp"),
            MathOpcode::DiscreteLog => write!(f, "math.discretelog"),
            MathOpcode::Jacobi => write!(f, "math.jacobi"),
            MathOpcode::Legendre => write!(f, "math.legendre"),
            MathOpcode::Mean => write!(f, "stat.mean"),
            MathOpcode::GeometricMean => write!(f, "stat.geomean"),
            MathOpcode::HarmonicMean => write!(f, "stat.harmean"),
            MathOpcode::Variance => write!(f, "stat.variance"),
            MathOpcode::StandardDeviation => write!(f, "stat.stddev"),
            MathOpcode::Skewness => write!(f, "stat.skewness"),
            MathOpcode::Kurtosis => write!(f, "stat.kurtosis"),
            MathOpcode::Correlation => write!(f, "stat.correlation"),
            MathOpcode::Covariance => write!(f, "stat.covariance"),
            MathOpcode::Determinant => write!(f, "linalg.det"),
            MathOpcode::Trace => write!(f, "linalg.trace"),
            MathOpcode::Rank => write!(f, "linalg.rank"),
            MathOpcode::Eigenvalues => write!(f, "linalg.eigenvals"),
            MathOpcode::Eigenvectors => write!(f, "linalg.eigenvecs"),
            MathOpcode::SingularValues => write!(f, "linalg.svd"),
            MathOpcode::Minimize => write!(f, "opt.minimize"),
            MathOpcode::Maximize => write!(f, "opt.maximize"),
            MathOpcode::FindRoot => write!(f, "opt.findroot"),
            MathOpcode::Integrate => write!(f, "calc.integrate"),
            MathOpcode::Differentiate => write!(f, "calc.differentiate"),
            MathOpcode::SetPrecision => write!(f, "prec.set"),
            MathOpcode::GetPrecision => write!(f, "prec.get"),
            MathOpcode::RoundToNearest => write!(f, "prec.round"),
            MathOpcode::RoundUp => write!(f, "prec.ceil"),
            MathOpcode::RoundDown => write!(f, "prec.floor"),
            MathOpcode::Truncate => write!(f, "prec.trunc"),
        }
    }
}

impl From<u8> for MathOpcode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => MathOpcode::HighPrecisionAdd,
            0x02 => MathOpcode::HighPrecisionSub,
            0x03 => MathOpcode::HighPrecisionMul,
            0x04 => MathOpcode::HighPrecisionDiv,
            0x05 => MathOpcode::HighPrecisionMod,
            0x10 => MathOpcode::HighPrecisionSqrt,
            0x11 => MathOpcode::HighPrecisionPow,
            0x12 => MathOpcode::HighPrecisionExp,
            0x13 => MathOpcode::HighPrecisionLog,
            0x14 => MathOpcode::HighPrecisionLog10,
            0x15 => MathOpcode::HighPrecisionLog2,
            0x20 => MathOpcode::HighPrecisionSin,
            0x21 => MathOpcode::HighPrecisionCos,
            0x22 => MathOpcode::HighPrecisionTan,
            0x23 => MathOpcode::HighPrecisionAsin,
            0x24 => MathOpcode::HighPrecisionAcos,
            0x25 => MathOpcode::HighPrecisionAtan,
            0x26 => MathOpcode::HighPrecisionAtan2,
            0x30 => MathOpcode::HighPrecisionSinh,
            0x31 => MathOpcode::HighPrecisionCosh,
            0x32 => MathOpcode::HighPrecisionTanh,
            0x33 => MathOpcode::HighPrecisionAsinh,
            0x34 => MathOpcode::HighPrecisionAcosh,
            0x35 => MathOpcode::HighPrecisionAtanh,
            0x40 => MathOpcode::Gamma,
            0x41 => MathOpcode::LogGamma,
            0x42 => MathOpcode::Beta,
            0x43 => MathOpcode::Erf,
            0x44 => MathOpcode::Erfc,
            0x45 => MathOpcode::Bessel,
            0x50 => MathOpcode::ModularExp,
            0x51 => MathOpcode::DiscreteLog,
            0x52 => MathOpcode::Jacobi,
            0x53 => MathOpcode::Legendre,
            0x60 => MathOpcode::Mean,
            0x61 => MathOpcode::GeometricMean,
            0x62 => MathOpcode::HarmonicMean,
            0x63 => MathOpcode::Variance,
            0x64 => MathOpcode::StandardDeviation,
            0x65 => MathOpcode::Skewness,
            0x66 => MathOpcode::Kurtosis,
            0x67 => MathOpcode::Correlation,
            0x68 => MathOpcode::Covariance,
            0x70 => MathOpcode::Determinant,
            0x71 => MathOpcode::Trace,
            0x72 => MathOpcode::Rank,
            0x73 => MathOpcode::Eigenvalues,
            0x74 => MathOpcode::Eigenvectors,
            0x75 => MathOpcode::SingularValues,
            0x80 => MathOpcode::Minimize,
            0x81 => MathOpcode::Maximize,
            0x82 => MathOpcode::FindRoot,
            0x83 => MathOpcode::Integrate,
            0x84 => MathOpcode::Differentiate,
            0x90 => MathOpcode::SetPrecision,
            0x91 => MathOpcode::GetPrecision,
            0x92 => MathOpcode::RoundToNearest,
            0x93 => MathOpcode::RoundUp,
            0x94 => MathOpcode::RoundDown,
            0x95 => MathOpcode::Truncate,
            _ => MathOpcode::HighPrecisionAdd, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_counts() {
        assert_eq!(MathOpcode::HighPrecisionAdd.operand_count(), 2);
        assert_eq!(MathOpcode::HighPrecisionSqrt.operand_count(), 1);
        assert_eq!(MathOpcode::ModularExp.operand_count(), 3);
    }

    #[test]
    fn test_precision_requirements() {
        assert_eq!(MathOpcode::HighPrecisionAdd.minimum_precision(), 128);
        assert_eq!(MathOpcode::HighPrecisionSin.minimum_precision(), 512);
        assert_eq!(MathOpcode::Gamma.minimum_precision(), 1024);
    }

    #[test]
    fn test_commutativity() {
        assert!(MathOpcode::HighPrecisionAdd.is_commutative());
        assert!(MathOpcode::HighPrecisionMul.is_commutative());
        assert!(!MathOpcode::HighPrecisionSub.is_commutative());
        assert!(!MathOpcode::HighPrecisionDiv.is_commutative());
    }

    #[test]
    fn test_complexity() {
        assert_eq!(MathOpcode::HighPrecisionAdd.complexity(), ComputationalComplexity::Linear);
        assert_eq!(MathOpcode::HighPrecisionMul.complexity(), ComputationalComplexity::Quadratic);
        assert_eq!(MathOpcode::Eigenvalues.complexity(), ComputationalComplexity::SuperExponential);
    }

    #[test]
    fn test_display() {
        assert_eq!(MathOpcode::HighPrecisionAdd.to_string(), "hp.add");
        assert_eq!(MathOpcode::Gamma.to_string(), "math.gamma");
        assert_eq!(MathOpcode::Mean.to_string(), "stat.mean");
    }
}