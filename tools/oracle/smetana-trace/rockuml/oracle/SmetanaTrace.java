package rockuml.oracle;

import static gen.lib.cgraph.edge__c.agfstout;
import static gen.lib.cgraph.edge__c.agnxtout;
import static gen.lib.cgraph.node__c.agfstnode;
import static gen.lib.cgraph.node__c.agnxtnode;
import static smetana.core.Macro.AGRAPH;
import static smetana.core.Macro.AGNODE;
import static smetana.core.Macro.ED_head_label;
import static smetana.core.Macro.ED_label;
import static smetana.core.Macro.ED_spl;
import static smetana.core.Macro.ED_tail_label;
import static smetana.core.Macro.ED_xlabel;
import static smetana.core.Macro.GD_bb;
import static smetana.core.Macro.GD_clust;
import static smetana.core.Macro.GD_label;
import static smetana.core.Macro.GD_maxrank;
import static smetana.core.Macro.GD_minrank;
import static smetana.core.Macro.GD_n_cluster;
import static smetana.core.Macro.GD_rank;
import static smetana.core.Macro.ND_coord;
import static smetana.core.Macro.ND_height;
import static smetana.core.Macro.ND_ht;
import static smetana.core.Macro.ND_lw;
import static smetana.core.Macro.ND_rank;
import static smetana.core.Macro.ND_rw;
import static smetana.core.Macro.ND_width;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Consumer;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import h.ST_Agedge_s;
import h.ST_Agnode_s;
import h.ST_Agobj_s;
import h.ST_Agraph_s;
import h.ST_bezier;
import h.ST_boxf;
import h.ST_pointf;
import h.ST_rank_t;
import h.ST_splines;
import h.ST_textlabel_t;
import smetana.core.CString;
import smetana.core.Globals;

/**
 * Writes one trace file per Smetana layout when the environment variable {@code ROCKUML_SMETANA_TRACE} names a
 * directory: the cgraph calls that built the graph, then the layout state after each dot phase and the final state
 * PlantUML reads back. tools/oracle/README.md documents the format.
 * <p>
 * The hooks are called from the reference sources through tools/oracle/smetana-trace/hooks.patch. Without the
 * environment variable they return at once, so goldens are unaffected.
 */
public final class SmetanaTrace {
	private static final String DIRECTORY = System.getenv("ROCKUML_SMETANA_TRACE");
	// Same pattern as Macro.hackInitDimensionFromLabel, which turns such labels into fixed label sizes.
	private static final Pattern DIMENSION_LABEL = Pattern.compile("_dim_([.\\d]+)_([\\d.]+)_");
	private static final AtomicInteger layouts = new AtomicInteger();
	// A layout's calls all carry the Globals it was opened with; nested layouts each get their own.
	private static final Map<Globals, Recorder> recorders = Collections.synchronizedMap(new IdentityHashMap<>());

	private SmetanaTrace() {
	}

	public static void agopen(Globals zz, CString name) {
		// gvContext opens cgraph's nameless ProtoGraph internally; it is not a layout.
		if (DIRECTORY == null || name == null)
			return;
		final Recorder recorder = new Recorder(zz, layouts.incrementAndGet());
		recorders.put(zz, recorder);
		recorder.line("agopen " + quote(name.getContent()));
	}

	public static void agsubg(Globals zz, ST_Agraph_s g, CString name) {
		ifRecording(zz, recorder -> recorder.line("agsubg " + recorder.object(g) + " " + quote(name.getContent())));
	}

	public static void agnode(Globals zz, ST_Agraph_s g, CString name) {
		ifRecording(zz, recorder -> recorder.line("agnode " + recorder.object(g) + " " + quote(name.getContent())));
	}

	public static void agedge(Globals zz, ST_Agraph_s g, ST_Agnode_s tail, ST_Agnode_s head, CString name) {
		ifRecording(zz, recorder -> recorder.line("agedge " + recorder.object(g) + " " + recorder.object(tail) + " "
				+ recorder.object(head) + (name == null ? "" : " " + quote(name.getContent()))));
	}

	public static void agsafeset(Globals zz, ST_Agobj_s obj, CString name, CString value, CString def) {
		ifRecording(zz, recorder -> recorder.line("agsafeset " + recorder.object(obj) + " " + quote(name.getContent())
				+ " " + attributeValue(value) + " " + attributeValue(def)));
	}

	public static void gvContext(Globals zz) {
		ifRecording(zz, recorder -> recorder.line("gvContext"));
	}

	public static void gvLayoutJobs(Globals zz, ST_Agraph_s g) {
		ifRecording(zz, recorder -> {
			recorder.line("gvLayoutJobs " + recorder.object(g));
			recorder.collectGraph(g);
		});
	}

	public static void afterRank(Globals zz, ST_Agraph_s g) {
		ifRecording(zz, recorder -> recorder.dumpRanks(g));
	}

	public static void afterMincross(Globals zz, ST_Agraph_s g) {
		ifRecording(zz, recorder -> recorder.dumpOrders(g));
	}

	public static void afterPosition(Globals zz, ST_Agraph_s g) {
		ifRecording(zz, recorder -> recorder.dumpPositions(g));
	}

	public static void afterSplines(Globals zz) {
		ifRecording(zz, recorder -> recorder.dumpSplines());
	}

	public static void afterLayout(Globals zz, ST_Agraph_s g) {
		ifRecording(zz, recorder -> {
			recorder.dumpFinal(g);
			recorder.write();
			recorders.remove(zz);
		});
	}

	private static void ifRecording(Globals zz, Consumer<Recorder> action) {
		if (DIRECTORY == null)
			return;
		final Recorder recorder = recorders.get(zz);
		if (recorder != null)
			action.accept(recorder);
	}

	private static String attributeValue(CString value) {
		final String content = value.getContent();
		final Matcher dimension = DIMENSION_LABEL.matcher(content);
		if (dimension.matches())
			return "dim(" + dimension.group(1) + "," + dimension.group(2) + ")";
		return quote(content);
	}

	private static String quote(String text) {
		final StringBuilder quoted = new StringBuilder("\"");
		for (char c : text.toCharArray()) {
			switch (c) {
			case '"':
				quoted.append("\\\"");
				break;
			case '\\':
				quoted.append("\\\\");
				break;
			case '\n':
				quoted.append("\\n");
				break;
			case '\r':
				quoted.append("\\r");
				break;
			case '\t':
				quoted.append("\\t");
				break;
			default:
				quoted.append(c);
			}
		}
		return quoted.append('"').toString();
	}

	// Double.toString prints the shortest decimal that parses back to the same double.
	private static String number(double value) {
		return Double.toString(value);
	}

	private static String point(ST_pointf p) {
		return number(p.x) + " " + number(p.y);
	}

	private static final class Recorder {
		private final Globals zz;
		private final int number;
		private final StringBuilder trace = new StringBuilder("smetana-trace 1\n");
		private final Map<ST_Agnode_s, String> nodeNames = new IdentityHashMap<>();
		private final List<ST_Agnode_s> nodes = new ArrayList<>();
		private final List<ST_Agedge_s> edges = new ArrayList<>();

		Recorder(Globals zz, int number) {
			this.zz = zz;
			this.number = number;
		}

		void line(String text) {
			trace.append(text).append('\n');
		}

		/** Graphs and nodes by name, edges by creation number (cgraph's sequence number). */
		String object(ST_Agobj_s obj) {
			switch (obj.tag.objtype) {
			case AGRAPH:
				return "graph " + quote(name(obj));
			case AGNODE:
				return "node " + quote(name(obj));
			default:
				return "edge " + edge((ST_Agedge_s) obj);
			}
		}

		// cgraph maps names to even ids through Globals.all; reading the map directly, unlike agnameof, leaves
		// cgraph's dictionaries untouched.
		private String name(ST_Agobj_s obj) {
			final CString name = obj.tag.id % 2 == 0 ? zz.all.get(obj.tag.id) : null;
			if (name == null)
				throw new IllegalStateException("anonymous " + obj.tag.objtype + " " + obj.tag.id);
			return name.getContent();
		}

		private static String edge(ST_Agedge_s e) {
			return "e" + e.tag.seq;
		}

		void collectGraph(ST_Agraph_s g) {
			for (ST_Agnode_s n = agfstnode(zz, g); n != null; n = agnxtnode(zz, g, n)) {
				nodes.add(n);
				nodeNames.put(n, quote(name(n)));
				for (ST_Agedge_s e = agfstout(zz, g, n); e != null; e = agnxtout(zz, g, e))
					edges.add(e);
			}
			edges.sort(Comparator.comparingInt(e -> e.tag.seq));
			for (int i = 0; i < edges.size(); i++)
				if (edges.get(i).tag.seq != i + 1)
					throw new IllegalStateException("edges are not numbered 1.." + edges.size());
		}

		/** Real nodes by quoted name; virtual nodes as v1, v2... in the order the trace first meets them. */
		private String node(ST_Agnode_s n) {
			return nodeNames.computeIfAbsent(n, virtual -> "v" + (nodeNames.size() - nodes.size() + 1));
		}

		void dumpRanks(ST_Agraph_s g) {
			line("phase rank");
			forEachGraph(g, graph -> line("graph " + quote(name(graph)) + " minrank " + GD_minrank(graph)
					+ " maxrank " + GD_maxrank(graph)));
			for (ST_Agnode_s n : nodes)
				line("node " + node(n) + " rank " + ND_rank(n));
		}

		void dumpOrders(ST_Agraph_s g) {
			line("phase mincross");
			for (int r = GD_minrank(g); r <= GD_maxrank(g); r++) {
				final StringBuilder rank = new StringBuilder("rank " + r);
				for (ST_Agnode_s n : rank(g, r))
					rank.append(' ').append(node(n));
				line(rank.toString());
			}
		}

		void dumpPositions(ST_Agraph_s g) {
			line("phase position");
			for (int r = GD_minrank(g); r <= GD_maxrank(g); r++)
				for (ST_Agnode_s n : rank(g, r))
					line("node " + node(n) + " rank " + r + " coord " + point(ND_coord(n)) + " lw " + number(ND_lw(n))
							+ " rw " + number(ND_rw(n)) + " ht " + number(ND_ht(n)));
			dumpGraphBoxes(g);
		}

		void dumpSplines() {
			line("phase splines");
			dumpEdges();
		}

		void dumpFinal(ST_Agraph_s g) {
			line("phase final");
			dumpGraphBoxes(g);
			for (ST_Agnode_s n : nodes)
				line("node " + node(n) + " coord " + point(ND_coord(n)) + " width " + number(ND_width(n)) + " height "
						+ number(ND_height(n)) + " lw " + number(ND_lw(n)) + " rw " + number(ND_rw(n)) + " ht "
						+ number(ND_ht(n)));
			dumpEdges();
		}

		private void dumpGraphBoxes(ST_Agraph_s g) {
			forEachGraph(g, graph -> {
				final ST_boxf bb = GD_bb(graph);
				final String ref = "graph " + quote(name(graph));
				line(ref + " bb " + point(bb.LL) + " " + point(bb.UR));
				dumpLabel(ref, "label", GD_label(graph));
			});
		}

		private void dumpEdges() {
			for (ST_Agedge_s e : edges) {
				final String ref = "edge " + edge(e);
				final ST_splines spl = ED_spl(e);
				if (spl == null)
					line(ref + " spl none");
				else
					for (int i = 0; i < spl.size; i++)
						line(ref + " bezier " + i + " " + bezier(spl.list.get__(i)));
				dumpLabel(ref, "label", ED_label(e));
				dumpLabel(ref, "head_label", ED_head_label(e));
				dumpLabel(ref, "tail_label", ED_tail_label(e));
				dumpLabel(ref, "xlabel", ED_xlabel(e));
			}
		}

		private static String bezier(ST_bezier bezier) {
			final StringBuilder text = new StringBuilder("sflag " + bezier.sflag + " eflag " + bezier.eflag + " sp "
					+ point(bezier.sp) + " ep " + point(bezier.ep) + " points " + bezier.size);
			for (int i = 0; i < bezier.size; i++)
				text.append(' ').append(point(bezier.list.get__(i)));
			return text.toString();
		}

		private void dumpLabel(String ref, String kind, ST_textlabel_t label) {
			if (label != null)
				line(ref + " " + kind + " pos " + point(label.pos) + " dimen " + point(label.dimen) + " set "
						+ label.set);
		}

		private static List<ST_Agnode_s> rank(ST_Agraph_s g, int r) {
			final ST_rank_t rank = GD_rank(g).get__(r);
			final List<ST_Agnode_s> members = new ArrayList<>();
			for (int i = 0; i < rank.n; i++)
				members.add(rank.v.get_(i));
			return members;
		}

		/** The graph, then its clusters depth-first in cluster-array order. */
		private static void forEachGraph(ST_Agraph_s g, Consumer<ST_Agraph_s> action) {
			action.accept(g);
			for (int c = 1; c <= GD_n_cluster(g); c++)
				forEachGraph(GD_clust(g).get_(c), action);
		}

		void write() {
			final Path file = Paths.get(DIRECTORY, String.format("%02d.trace", number));
			try {
				Files.createDirectories(file.getParent());
				Files.write(file, trace.toString().getBytes(StandardCharsets.UTF_8));
			} catch (IOException e) {
				throw new UncheckedIOException(e);
			}
		}
	}
}
