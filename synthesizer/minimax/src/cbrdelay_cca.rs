// The CCA engine is shared with the historical `cc` binary.  Keeping this
// named entrypoint makes the CBR-delay experiment discoverable without
// duplicating the search implementation.
include!("cong_ctrl.rs");
