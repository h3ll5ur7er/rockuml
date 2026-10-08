//! Activity diagrams squeeze the empty space out of their drawing, first across, then down (PlantUML's
//! `klimt.compress`).
//!
//! Only the axis a pass works on lives here so far. The passes themselves (`SlotFinder`, `SlotSet`, `Slot`,
//! `CompressionTransform`, `CompressionXorYBuilder` and the `UGraphicCompressOnXorY` layer) belong in this
//! module too: the layer is a [`super::ugraphic::UGraphicLayer`] that keeps its own translation, maps the
//! primitives it passes on, and draws the text of [`super::shape::UShape::CenteredText`], which no output
//! format draws. `SlotFinder` is a backend: it receives absolute positions, as `LimitFinder` does.

/// The axis a compression pass works on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompressionMode {
    OnX,
    #[allow(dead_code, reason = "the pass down, Phase 6 stage E1")]
    OnY,
}
