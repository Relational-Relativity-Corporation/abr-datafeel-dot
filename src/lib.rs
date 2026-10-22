// abr-datafeel-dot
// Metatron Dynamics, Inc. — Delaware C-Corp #10551645
//
// Application of Kernel V8 to the DataFeel Dot: thermal, haptic, and optical
// outputs governed by a single declared relational process rather than three
// independent sequences.
//
// Module layout:
//   device_interface   — DeviceInterface trait + command types + MockDevice
//   observation        — Observation type (timestamped, provenance-tagged)
//   declaration        — MDeclaration parser and relational topology types
//   relational_loop    — core loop: read → declare → evaluate → project
//   actuator_projection — RelationalOutput → bounded device commands

pub mod device_interface;
pub mod observation;
pub mod declaration;
pub mod relational_loop;
pub mod actuator_projection;
