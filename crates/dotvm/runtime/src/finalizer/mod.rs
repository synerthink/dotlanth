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

// Core modules for finality handling and auditing
pub mod finality_confirmation;
pub mod finality_validation;
pub mod instant_finality;
pub mod lib;
pub mod logging_audit;

// Public exports
pub use finality_confirmation::FinalityConfirmation;
pub use finality_validation::FinalityValidator;
pub use instant_finality::InstantFinalityModule;
pub use logging_audit::AuditLogger;
