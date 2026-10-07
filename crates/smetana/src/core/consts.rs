//! The constants of `Macro.java` that the dot phases share. Arrow, shape and module-private constants are
//! declared by the modules that use them.

/// Node types (`ND_node_type`) and edge types (`ED_edge_type`).
pub(crate) const NORMAL: i32 = 0;
pub const VIRTUAL: i32 = 1;
pub(crate) const SLACKNODE: i32 = 2;
pub(crate) const REVERSED: i32 = 3;
pub(crate) const FLATORDER: i32 = 4;
pub(crate) const CLUSTER_EDGE: i32 = 5;
pub(crate) const IGNORED: i32 = 6;

/// Collapsed node classifications (`ND_ranktype`).
pub(crate) const SAMERANK: i32 = 1;
pub(crate) const MINRANK: i32 = 2;
pub(crate) const SOURCERANK: i32 = 3;
pub(crate) const MAXRANK: i32 = 4;
pub(crate) const SINKRANK: i32 = 5;
pub(crate) const LEAFSET: i32 = 6;
pub(crate) const CLUSTER: i32 = 7;

/// `State` once dot has routed the edges.
pub(crate) const GVSPLINES: i32 = 1;

/// `GD_label_pos`.
pub(crate) const LABEL_AT_BOTTOM: i32 = 0;
pub(crate) const LABEL_AT_TOP: i32 = 1;
pub(crate) const LABEL_AT_LEFT: i32 = 2;
pub(crate) const LABEL_AT_RIGHT: i32 = 4;

/// `rankdir`.
pub(crate) const RANKDIR_TB: i32 = 0;
pub(crate) const RANKDIR_LR: i32 = 1;
pub(crate) const RANKDIR_BT: i32 = 2;
pub(crate) const RANKDIR_RL: i32 = 3;

/// Spline edge classes (`ED_tree_index` during splines).
pub(crate) const REGULAREDGE: i32 = 1;
pub(crate) const FLATEDGE: i32 = 2;

pub(crate) const SELFEDGE: i32 = 8;
pub(crate) const EDGETYPEMASK: i32 = 15;

/// Edge routing types (`GD_flags & ET_*`).
pub(crate) const ET_NONE: i32 = 0 << 1;
pub(crate) const ET_LINE: i32 = 1 << 1;
pub(crate) const ET_CURVED: i32 = 2 << 1;
pub(crate) const ET_PLINE: i32 = 3 << 1;

pub const ET_SPLINE: i32 = 5 << 1;

/// `NEW_RANK` in `GD_flags`.
pub(crate) const NEW_RANK: i32 = 1 << 4;

/// Label types.
pub(crate) const LT_NONE: i32 = 0 << 1;
pub(crate) const LT_HTML: i32 = 1 << 1;
pub(crate) const LT_RECD: i32 = 2 << 1;

/// Which labels exist (`GD_has_labels`).
pub(crate) const EDGE_LABEL: i32 = 1 << 0;
pub(crate) const HEAD_LABEL: i32 = 1 << 1;
pub(crate) const TAIL_LABEL: i32 = 1 << 2;
pub(crate) const GRAPH_LABEL: i32 = 1 << 3;
pub(crate) const NODE_XLABEL: i32 = 1 << 4;
pub(crate) const EDGE_XLABEL: i32 = 1 << 5;

/// Sides of a box.
pub(crate) const BOTTOM: i32 = 1 << 0;
pub(crate) const RIGHT: i32 = 1 << 1;
pub(crate) const TOP: i32 = 1 << 2;
pub(crate) const LEFT: i32 = 1 << 3;

/// Indices into cluster margins.
pub(crate) const BOTTOM_IX: i32 = 0;
pub(crate) const RIGHT_IX: i32 = 1;
pub(crate) const TOP_IX: i32 = 2;
pub(crate) const LEFT_IX: i32 = 3;

/// Cluster rank assignment (`CL_type`).
pub(crate) const LOCAL: i32 = 100;
pub(crate) const GLOBAL: i32 = 101;
pub(crate) const NOCLUST: i32 = 102;

/// The cluster margin in points, and the cost of a crossing with a cluster.
pub(crate) const CL_OFFSET: i32 = 8;
pub(crate) const CL_CROSS: i32 = 1000;

pub(crate) const GAP: i32 = 4;
pub(crate) const MAXSHORT: i32 = 0x7fff;
pub(crate) const INT_MAX: i32 = i32::MAX;
pub(crate) const INT_MIN: i32 = i32::MIN;
pub(crate) const USHRT_MAX: i32 = 65535;
pub(crate) const MILLIPOINT: f64 = 0.001;
pub(crate) const M_PI: f64 = std::f64::consts::PI;

pub(crate) const DEFAULT_NODESEP: f64 = 0.25;
pub(crate) const MIN_NODESEP: f64 = 0.02;
pub(crate) const DEFAULT_RANKSEP: f64 = 0.5;
pub(crate) const MIN_RANKSEP: f64 = 0.02;
pub(crate) const DEFAULT_NODEHEIGHT: f64 = 0.5;
pub(crate) const MIN_NODEHEIGHT: f64 = 0.02;
pub(crate) const DEFAULT_NODEWIDTH: f64 = 0.75;
pub(crate) const MIN_NODEWIDTH: f64 = 0.01;
pub(crate) const DEFAULT_FONTSIZE: f64 = 14.0;

pub(crate) const MIN_FONTSIZE: f64 = 1.0;
pub(crate) const NODENAME_ESC: &str = "\\N";
pub(crate) const DEFAULT_NODESHAPE: &str = "ellipse";
