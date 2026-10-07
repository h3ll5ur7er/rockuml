//! The constants of `Macro.java` that the dot phases share. Arrow, shape and module-private constants are
//! declared by the modules that use them.

/// Node types (`ND_node_type`) and edge types (`ED_edge_type`).
pub const NORMAL: i32 = 0;
pub const VIRTUAL: i32 = 1;
pub const SLACKNODE: i32 = 2;
pub const REVERSED: i32 = 3;
pub const FLATORDER: i32 = 4;
pub const CLUSTER_EDGE: i32 = 5;
pub const IGNORED: i32 = 6;

/// Collapsed node classifications (`ND_ranktype`).
pub const SAMERANK: i32 = 1;
pub const MINRANK: i32 = 2;
pub const SOURCERANK: i32 = 3;
pub const MAXRANK: i32 = 4;
pub const SINKRANK: i32 = 5;
pub const LEAFSET: i32 = 6;
pub const CLUSTER: i32 = 7;

/// `State` once dot has routed the edges.
pub const GVSPLINES: i32 = 1;

/// `GD_label_pos`.
pub const LABEL_AT_BOTTOM: i32 = 0;
pub const LABEL_AT_TOP: i32 = 1;
pub const LABEL_AT_LEFT: i32 = 2;
pub const LABEL_AT_RIGHT: i32 = 4;

/// `rankdir`.
pub const RANKDIR_TB: i32 = 0;
pub const RANKDIR_LR: i32 = 1;
pub const RANKDIR_BT: i32 = 2;
pub const RANKDIR_RL: i32 = 3;

/// Spline edge classes (`ED_tree_index` during splines).
pub const REGULAREDGE: i32 = 1;
pub const FLATEDGE: i32 = 2;

pub const SELFEDGE: i32 = 8;
pub const EDGETYPEMASK: i32 = 15;

/// Edge routing types (`GD_flags & ET_*`).
pub const ET_NONE: i32 = 0 << 1;
pub const ET_LINE: i32 = 1 << 1;
pub const ET_CURVED: i32 = 2 << 1;
pub const ET_PLINE: i32 = 3 << 1;

pub const ET_SPLINE: i32 = 5 << 1;

/// `NEW_RANK` in `GD_flags`.
pub const NEW_RANK: i32 = 1 << 4;

/// Label types.
pub const LT_NONE: i32 = 0 << 1;
pub const LT_HTML: i32 = 1 << 1;
pub const LT_RECD: i32 = 2 << 1;

/// Which labels exist (`GD_has_labels`).
pub const EDGE_LABEL: i32 = 1 << 0;
pub const HEAD_LABEL: i32 = 1 << 1;
pub const TAIL_LABEL: i32 = 1 << 2;
pub const GRAPH_LABEL: i32 = 1 << 3;
pub const NODE_XLABEL: i32 = 1 << 4;
pub const EDGE_XLABEL: i32 = 1 << 5;

/// Sides of a box.
pub const BOTTOM: i32 = 1 << 0;
pub const RIGHT: i32 = 1 << 1;
pub const TOP: i32 = 1 << 2;
pub const LEFT: i32 = 1 << 3;

/// Indices into cluster margins.
pub const BOTTOM_IX: i32 = 0;
pub const RIGHT_IX: i32 = 1;
pub const TOP_IX: i32 = 2;
pub const LEFT_IX: i32 = 3;

/// Cluster rank assignment (`CL_type`).
pub const LOCAL: i32 = 100;
pub const GLOBAL: i32 = 101;
pub const NOCLUST: i32 = 102;

/// The cluster margin in points, and the cost of a crossing with a cluster.
pub const CL_OFFSET: i32 = 8;
pub const CL_CROSS: i32 = 1000;

pub const GAP: i32 = 4;
pub const MAXSHORT: i32 = 0x7fff;
pub const INT_MAX: i32 = i32::MAX;
pub const INT_MIN: i32 = i32::MIN;
pub const USHRT_MAX: i32 = 65535;
pub const MILLIPOINT: f64 = 0.001;
pub const M_PI: f64 = std::f64::consts::PI;

pub const DEFAULT_NODESEP: f64 = 0.25;
pub const MIN_NODESEP: f64 = 0.02;
pub const DEFAULT_RANKSEP: f64 = 0.5;
pub const MIN_RANKSEP: f64 = 0.02;
pub const DEFAULT_NODEHEIGHT: f64 = 0.5;
pub const MIN_NODEHEIGHT: f64 = 0.02;
pub const DEFAULT_NODEWIDTH: f64 = 0.75;
pub const MIN_NODEWIDTH: f64 = 0.01;
pub const DEFAULT_FONTSIZE: f64 = 14.0;

pub const MIN_FONTSIZE: f64 = 1.0;
pub const NODENAME_ESC: &str = "\\N";
pub const DEFAULT_NODESHAPE: &str = "ellipse";
