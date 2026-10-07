import static gen.lib.cgraph.attr__c.agattr;
import static gen.lib.cgraph.attr__c.agget;
import static gen.lib.cgraph.attr__c.agsafeset;
import static gen.lib.cgraph.edge__c.agedge;
import static gen.lib.cgraph.edge__c.agfstedge;
import static gen.lib.cgraph.edge__c.agfstin;
import static gen.lib.cgraph.edge__c.agfstout;
import static gen.lib.cgraph.edge__c.agnxtedge;
import static gen.lib.cgraph.edge__c.agnxtin;
import static gen.lib.cgraph.edge__c.agnxtout;
import static gen.lib.cgraph.edge__c.aghead;
import static gen.lib.cgraph.edge__c.agtail;
import static gen.lib.cgraph.graph__c.agdegree;
import static gen.lib.cgraph.graph__c.agnedges;
import static gen.lib.cgraph.graph__c.agnnodes;
import static gen.lib.cgraph.graph__c.agopen;
import static gen.lib.cgraph.id__c.agnameof;
import static gen.lib.cgraph.node__c.agfstnode;
import static gen.lib.cgraph.node__c.agnode;
import static gen.lib.cgraph.node__c.agnxtnode;
import static gen.lib.cgraph.subg__c.agfstsubg;
import static gen.lib.cgraph.subg__c.agnxtsubg;
import static gen.lib.cgraph.subg__c.agsubg;
import static smetana.core.Macro.AGINEDGE;
import static smetana.core.Macro.AGNODE;
import static smetana.core.Macro.agfindedge;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import h.ST_Agedge_s;
import h.ST_Agnode_s;
import h.ST_Agobj_s;
import h.ST_Agraph_s;
import smetana.core.CString;
import smetana.core.Globals;

/**
 * Replays the input section of a Smetana trace through Java's cgraph and dumps what the port's cgraph must
 * reproduce: counts, iteration orders, id order, wildcard edge lookups and attribute values. Usage: CgraphDump
 * in.trace out.dump. crates/smetana/tests/cgraph_replay.rs makes the same calls in the same order (iterating and
 * searching reshape cgraph's splay trees) and writes the same format.
 */
public class CgraphDump {
	private final Globals zz = Globals.open();
	private ST_Agraph_s root;
	private final Map<String, ST_Agraph_s> graphs = new HashMap<>();
	private final Map<String, ST_Agnode_s> nodes = new HashMap<>();
	private final List<ST_Agedge_s> edges = new ArrayList<>();
	/** The (object, attribute) pairs agsafeset set, in first-seen order. */
	private final List<String[]> attributes = new ArrayList<>();

	public static void main(String[] args) throws IOException {
		final CgraphDump dump = new CgraphDump();
		for (String line : Files.readAllLines(Path.of(args[0]), StandardCharsets.UTF_8))
			if (line.startsWith("phase ") || !dump.call(tokens(line)))
				break;
		Files.writeString(Path.of(args[1]), dump.dump(), StandardCharsets.UTF_8);
	}

	static List<String> tokens(String line) {
		final List<String> out = new ArrayList<>();
		int i = 0;
		while (i < line.length()) {
			final char c = line.charAt(i);
			if (c == ' ') {
				i++;
			} else if (c == '"') {
				final StringBuilder s = new StringBuilder();
				i++;
				while (line.charAt(i) != '"') {
					char d = line.charAt(i++);
					if (d == '\\') {
						d = line.charAt(i++);
						d = d == 'n' ? '\n' : d == 'r' ? '\r' : d == 't' ? '\t' : d;
					}
					s.append(d);
				}
				i++;
				out.add(s.toString());
			} else {
				int end = line.indexOf(' ', i);
				if (end < 0)
					end = line.length();
				String word = line.substring(i, end);
				if (word.startsWith("dim(") && word.endsWith(")")) {
					final String[] wh = word.substring(4, word.length() - 1).split(",");
					word = "_dim_" + wh[0] + "_" + wh[1] + "_";
				}
				out.add(word);
				i = end;
			}
		}
		return out;
	}

	private ST_Agobj_s object(String kind, String name) {
		switch (kind) {
		case "graph":
			return graphs.get(name);
		case "node":
			return nodes.get(name);
		default:
			return edges.get(Integer.parseInt(name.substring(1)) - 1);
		}
	}

	/** Makes one input call; false at gvLayoutJobs. */
	private boolean call(List<String> t) {
		switch (t.get(0)) {
		case "smetana-trace":
			return true;
		case "agopen":
			root = agopen(zz, new CString(t.get(1)), zz.Agdirected, null);
			graphs.put(t.get(1), root);
			return true;
		case "agsubg":
			graphs.put(t.get(3), agsubg(zz, graphs.get(t.get(2)), new CString(t.get(3)), true));
			return true;
		case "agnode":
			nodes.put(t.get(3), agnode(zz, graphs.get(t.get(2)), new CString(t.get(3)), true));
			return true;
		case "agedge":
			edges.add(agedge(zz, graphs.get(t.get(2)), nodes.get(t.get(4)), nodes.get(t.get(6)),
					t.size() > 7 ? new CString(t.get(7)) : null, true));
			return true;
		case "agsafeset":
			agsafeset(zz, object(t.get(1), t.get(2)), new CString(t.get(3)), new CString(t.get(4)),
					new CString(t.get(5)));
			if (attributes.stream().noneMatch(a -> a[0].equals(t.get(1)) && a[1].equals(t.get(2))
					&& a[2].equals(t.get(3))))
				attributes.add(new String[] { t.get(1), t.get(2), t.get(3) });
			return true;
		case "gvContext":
			// What gvContext does to cgraph: declare the default node label on the prototype graph.
			agattr(zz, null, AGNODE, new CString("label"), new CString("\\N"));
			return true;
		case "gvLayoutJobs":
			return false;
		default:
			throw new IllegalArgumentException(t.toString());
		}
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

	private String name(ST_Agobj_s obj) {
		return quote(agnameof(zz, obj).getContent());
	}

	private static String edge(ST_Agedge_s e) {
		if (e == null)
			return "none";
		return "e" + e.tag.seq + (e.tag.objtype == AGINEDGE ? "<" : ">");
	}

	private List<ST_Agnode_s> nodes(ST_Agraph_s g) {
		final List<ST_Agnode_s> out = new ArrayList<>();
		for (ST_Agnode_s n = agfstnode(zz, g); n != null; n = agnxtnode(zz, g, n))
			out.add(n);
		return out;
	}

	private void subgraphs(ST_Agraph_s g, List<ST_Agraph_s> out) {
		for (ST_Agraph_s s = agfstsubg(zz, g); s != null; s = agnxtsubg(zz, s)) {
			out.add(s);
			subgraphs(s, out);
		}
	}

	private String edgeLists(ST_Agraph_s g, ST_Agnode_s n) {
		final StringBuilder line = new StringBuilder(" out");
		for (ST_Agedge_s e = agfstout(zz, g, n); e != null; e = agnxtout(zz, g, e))
			line.append(' ').append(edge(e));
		line.append(" in");
		for (ST_Agedge_s e = agfstin(zz, g, n); e != null; e = agnxtin(zz, g, e))
			line.append(' ').append(edge(e));
		line.append(" edges");
		for (ST_Agedge_s e = agfstedge(zz, g, n); e != null; e = agnxtedge(zz, g, e, n))
			line.append(' ').append(edge(e));
		final int din = agdegree(zz, g, n, true, false);
		final int dout = agdegree(zz, g, n, false, true);
		return line.append(" degree ").append(din).append(' ').append(dout).toString();
	}

	private String dump() {
		final StringBuilder out = new StringBuilder("cgraph-dump 1\n");
		final int nn = agnnodes(root);
		final int ne = agnedges(zz, root);
		out.append("graph ").append(name(root)).append(" nodes ").append(nn).append(" edges ").append(ne).append('\n');
		final List<ST_Agraph_s> subs = new ArrayList<>();
		subgraphs(root, subs);
		for (ST_Agraph_s s : subs) {
			out.append("subgraph ").append(name(s)).append(" parent ").append(name(s.parent)).append(" seq ")
					.append(s.tag.seq).append(" nodes");
			for (ST_Agnode_s n : nodes(s))
				out.append(' ').append(name(n));
			out.append('\n');
		}
		final List<ST_Agraph_s> all = new ArrayList<>();
		all.add(root);
		all.addAll(subs);
		final List<ST_Agraph_s> byId = new ArrayList<>(all);
		byId.sort(Comparator.comparingInt(g -> g.tag.id));
		out.append("graph ids");
		for (ST_Agraph_s g : byId)
			out.append(' ').append(name(g));
		final List<ST_Agnode_s> nodesById = nodes(root);
		nodesById.sort(Comparator.comparingInt(n -> n.tag.id));
		out.append("\nnode ids");
		for (ST_Agnode_s n : nodesById)
			out.append(' ').append(name(n));
		out.append('\n');
		for (ST_Agraph_s g : all)
			for (ST_Agnode_s n : nodes(g))
				out.append("in ").append(name(g)).append(" node ").append(name(n)).append(edgeLists(g, n)).append('\n');
		for (ST_Agedge_s e : edges) {
			final ST_Agnode_s t = agtail(e);
			final ST_Agnode_s h = aghead(e);
			final ST_Agedge_s forward = agfindedge(zz, root, t, h);
			final ST_Agedge_s backward = agfindedge(zz, root, h, t);
			out.append("find ").append(edge(e)).append(" forward ").append(edge(forward)).append(" backward ")
					.append(edge(backward)).append('\n');
		}
		for (String[] a : attributes) {
			final ST_Agobj_s obj = object(a[0], a[1]);
			final CString value = agget(zz, obj, new CString(a[2]));
			final String label = a[0].equals("edge") ? edge((ST_Agedge_s) obj) : name(obj);
			out.append("attr ").append(label).append(' ').append(quote(a[2])).append(' ')
					.append(value == null ? "null" : quote(value.getContent())).append('\n');
		}
		return out.toString();
	}
}
