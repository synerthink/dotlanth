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

use crate::bytecode::{BytecodeHeader, VmArchitecture};
// Unused Arch imports removed, they are not directly used in this file anymore
// after get_arch_impl was removed. VmArchitecture enum is used instead.
// use crate::memory::{Architecture, Arch32, Arch64, Arch128, Arch256, Arch512};

/// Represents the detected architecture requirement and compatibility.
#[derive(Debug, PartialEq, Eq)]
pub struct DetectedArch {
    /// The architecture required by the bytecode.
    pub required: VmArchitecture,
    /// The architecture that will be used for execution (could be the same as required or a compatible higher arch).
    pub execution: VmArchitecture,
    /// Indicates if the execution architecture is different from the required one (i.e., running in compatibility mode).
    pub compatibility_mode: bool,
}

/// Errors that can occur during architecture detection.
#[derive(Debug, PartialEq, Eq)]
pub enum ArchDetectionError {
    /// Bytecode is too short to contain a valid header.
    BytecodeTooShort,
    /// Error parsing the bytecode header.
    HeaderParseError(&'static str),
    /// The bytecode requires an architecture not supported by this VM.
    UnsupportedArchitecture(VmArchitecture),
    /// The bytecode requires a higher architecture than the VM's current maximum capability.
    RequiresHigherArch(VmArchitecture),
}

/// Analyzes bytecode to detect the required architecture and determine compatibility.
pub struct ArchitectureDetector {
    // In the future, this might hold VM's capability limits, e.g., max supported architecture.
    // For now, we assume the VM can run any of the VmArchitecture types if explicitly chosen.
}

impl Default for ArchitectureDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchitectureDetector {
    pub fn new() -> Self {
        ArchitectureDetector {}
    }

    /// Detects the architecture from bytecode.
    ///
    /// # Arguments
    /// * `bytecode`: A slice of bytes representing the program bytecode.
    /// * `preferred_arch`: Optionally, a preferred architecture to run on, if compatible.
    ///                     If None, the system will try to match the bytecode's requirement or use a compatible one.
    ///
    /// # Returns
    /// A `Result` containing the `DetectedArch` or an `ArchDetectionError`.
    pub fn detect(&self, bytecode: &[u8], preferred_arch: Option<VmArchitecture>) -> Result<DetectedArch, ArchDetectionError> {
        if bytecode.len() < BytecodeHeader::size() {
            return Err(ArchDetectionError::BytecodeTooShort);
        }

        let header = BytecodeHeader::from_bytes(&bytecode[0..BytecodeHeader::size()]).map_err(ArchDetectionError::HeaderParseError)?;

        let required_arch = header.architecture;

        // Determine the execution architecture
        let (execution_arch, compatibility_mode) = self.determine_execution_arch(required_arch, preferred_arch)?;

        Ok(DetectedArch {
            required: required_arch,
            execution: execution_arch,
            compatibility_mode,
        })
    }

    /// Determines the actual execution architecture based on bytecode requirements and VM capabilities/preferences.
    /// For now, "VM capabilities" means it can run any defined VmArchitecture.
    /// The compatibility rule is: a higher-bit architecture can run lower-bit bytecode.
    /// E.g., Arch256 can run Arch128, Arch64, Arch32. Arch128 cannot run Arch256.
    fn determine_execution_arch(&self, required: VmArchitecture, preferred: Option<VmArchitecture>) -> Result<(VmArchitecture, bool), ArchDetectionError> {
        let chosen_arch = match preferred {
            Some(pref_arch) => {
                // If a preference is given, check if it's compatible
                if self.is_compatible(required, pref_arch) {
                    pref_arch
                } else {
                    // Preferred architecture cannot run the required bytecode (e.g. trying to run 256-bit code on 64-bit VM)
                    return Err(ArchDetectionError::RequiresHigherArch(required));
                }
            }
            None => {
                // No preference, use the required architecture directly
                required
            }
        };

        let compatibility_mode = chosen_arch != required;
        Ok((chosen_arch, compatibility_mode))
    }

    /// Checks if `target_vm_arch` can run bytecode designed for `bytecode_arch`.
    /// Compatibility: A higher-bit architecture can run lower-bit bytecode.
    /// e.g., Arch256 (target_vm_arch) can run Arch128 (bytecode_arch).
    pub fn is_compatible(&self, bytecode_arch: VmArchitecture, target_vm_arch: VmArchitecture) -> bool {
        target_vm_arch.word_size() >= bytecode_arch.word_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::BytecodeHeader;

    fn create_test_bytecode(arch: VmArchitecture) -> Vec<u8> {
        let header = BytecodeHeader::new(arch);
        let mut bytecode = header.to_bytes().to_vec();
        bytecode.extend_from_slice(&[0u8; 10]); // Add some dummy payload
        bytecode
    }

    #[test]
    fn test_detect_exact_match_no_preference() {
        let detector = ArchitectureDetector::new();
        let bytecode = create_test_bytecode(VmArchitecture::Arch64);
        let result = detector.detect(&bytecode, None).unwrap();
        assert_eq!(
            result,
            DetectedArch {
                required: VmArchitecture::Arch64,
                execution: VmArchitecture::Arch64,
                compatibility_mode: false,
            }
        );
    }

    #[test]
    fn test_detect_exact_match_with_preference() {
        let detector = ArchitectureDetector::new();
        let bytecode = create_test_bytecode(VmArchitecture::Arch128);
        let result = detector.detect(&bytecode, Some(VmArchitecture::Arch128)).unwrap();
        assert_eq!(
            result,
            DetectedArch {
                required: VmArchitecture::Arch128,
                execution: VmArchitecture::Arch128,
                compatibility_mode: false,
            }
        );
    }

    #[test]
    fn test_detect_compatibility_mode_no_preference() {
        // This case currently doesn't trigger compatibility mode without preference,
        // as it defaults to required. It will be more relevant when VM has its own "max_arch" or "default_arch".
        // For now, it behaves like exact match.
        let detector = ArchitectureDetector::new();
        let bytecode = create_test_bytecode(VmArchitecture::Arch32);
        let result = detector.detect(&bytecode, None).unwrap();
        assert_eq!(
            result,
            DetectedArch {
                required: VmArchitecture::Arch32,
                execution: VmArchitecture::Arch32,
                compatibility_mode: false,
            }
        );
    }

    #[test]
    fn test_detect_compatibility_mode_with_higher_preference() {
        let detector = ArchitectureDetector::new();
        let bytecode = create_test_bytecode(VmArchitecture::Arch64);
        // Prefer to run 64-bit code on a 256-bit capable VM
        let result = detector.detect(&bytecode, Some(VmArchitecture::Arch256)).unwrap();
        assert_eq!(
            result,
            DetectedArch {
                required: VmArchitecture::Arch64,
                execution: VmArchitecture::Arch256,
                compatibility_mode: true,
            }
        );
    }

    #[test]
    fn test_detect_error_bytecode_too_short() {
        let detector = ArchitectureDetector::new();
        let bytecode = &create_test_bytecode(VmArchitecture::Arch64)[0..5]; // Too short
        let result = detector.detect(bytecode, None);
        assert_eq!(result, Err(ArchDetectionError::BytecodeTooShort));
    }

    #[test]
    fn test_detect_error_header_parse() {
        let detector = ArchitectureDetector::new();
        let mut bytecode = create_test_bytecode(VmArchitecture::Arch64);
        bytecode[0] = b'X'; // Corrupt magic number
        let result = detector.detect(&bytecode, None);
        assert_eq!(result, Err(ArchDetectionError::HeaderParseError("Invalid magic number")));
    }

    #[test]
    fn test_detect_error_preference_requires_higher_arch() {
        let detector = ArchitectureDetector::new();
        let bytecode = create_test_bytecode(VmArchitecture::Arch256);
        // Trying to run 256-bit code on a 64-bit preferred VM
        let result = detector.detect(&bytecode, Some(VmArchitecture::Arch64));
        assert_eq!(result, Err(ArchDetectionError::RequiresHigherArch(VmArchitecture::Arch256)));
    }

    #[test]
    fn test_is_compatible() {
        let detector = ArchitectureDetector::new();
        // Exact match
        assert!(detector.is_compatible(VmArchitecture::Arch64, VmArchitecture::Arch64));
        // Higher VM can run lower bytecode
        assert!(detector.is_compatible(VmArchitecture::Arch32, VmArchitecture::Arch64));
        assert!(detector.is_compatible(VmArchitecture::Arch64, VmArchitecture::Arch128));
        assert!(detector.is_compatible(VmArchitecture::Arch128, VmArchitecture::Arch256));
        assert!(detector.is_compatible(VmArchitecture::Arch256, VmArchitecture::Arch512));
        assert!(detector.is_compatible(VmArchitecture::Arch32, VmArchitecture::Arch512));
        // Lower VM cannot run higher bytecode
        assert!(!detector.is_compatible(VmArchitecture::Arch64, VmArchitecture::Arch32));
        assert!(!detector.is_compatible(VmArchitecture::Arch128, VmArchitecture::Arch64));
        assert!(!detector.is_compatible(VmArchitecture::Arch256, VmArchitecture::Arch128));
        assert!(!detector.is_compatible(VmArchitecture::Arch512, VmArchitecture::Arch256));
        assert!(!detector.is_compatible(VmArchitecture::Arch512, VmArchitecture::Arch32));
    }

    // The get_arch_impl method was removed, so this test is no longer needed.
    // #[test]
    // fn test_get_arch_impl_types() {
    //     let arch32 = ArchitectureDetector::get_arch_impl(VmArchitecture::Arch32);
    //     assert_eq!(arch32.word_size(), Arch32::WORD_SIZE);

    //     let arch64 = ArchitectureDetector::get_arch_impl(VmArchitecture::Arch64);
    //     assert_eq!(arch64.word_size(), Arch64::WORD_SIZE);

    //     let arch128 = ArchitectureDetector::get_arch_impl(VmArchitecture::Arch128);
    //     assert_eq!(arch128.word_size(), Arch128::WORD_SIZE);

    //     let arch256 = ArchitectureDetector::get_arch_impl(VmArchitecture::Arch256);
    //     assert_eq!(arch256.word_size(), Arch256::WORD_SIZE);

    //     let arch512 = ArchitectureDetector::get_arch_impl(VmArchitecture::Arch512);
    //     assert_eq!(arch512.word_size(), Arch512::WORD_SIZE);
    // }
}
