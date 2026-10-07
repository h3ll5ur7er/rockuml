package rockuml.oracle;

import static gen.lib.cgraph.attr__c.agsafeset;
import static gen.lib.cgraph.edge__c.agedge;
import static gen.lib.cgraph.graph__c.agopen;
import static gen.lib.cgraph.node__c.agnode;
import static gen.lib.cgraph.subg__c.agsubg;
import static gen.lib.gvc.gvc__c.gvContext;
import static gen.lib.gvc.gvlayout__c.gvLayoutJobs;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;
import java.util.TreeSet;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import h.ST_Agnode_s;
import h.ST_Agobj_s;
import h.ST_Agraph_s;
import h.ST_GVC_s;
import smetana.core.CString;
import smetana.core.Globals;
import smetana.core.Macro;

/**
 * Lays out reproducible random graphs with Smetana, so that SmetanaTrace records a layout trace for each.
 * <p>
 * The graphs are built through the cgraph calls PlantUML makes, with attribute values PlantUML could set, in one of
 * three styles: CucaDiagramFileMakerSmetana (class and description diagrams), SmetanaForGit and SmetanaForJson.
 * Seed {@code s} always gives the same graph; low seeds give small, plain graphs and features get denser up to
 * seed {@link #FULL_DENSITY_SEED}.
 * <p>
 * Usage: {@code RandomGraphs <output directory> <first seed> <last seed>}, with {@code ROCKUML_SMETANA_TRACE}
 * naming an empty staging directory. Each laid-out graph's trace is moved to {@code <output>/<seed>.trace}, and one
 * summary line per seed is printed: {@code <seed> ok <style> nodes N edges E [features]} or
 * {@code <seed> skipped <reason>}.
 */
public final class RandomGraphs {
	private static final int FULL_DENSITY_SEED = 200;
	private static final int MAX_NODES = 40;
	private static final Pattern VIRTUAL_NODE = Pattern.compile("(?m) v1( |$)");

	private RandomGraphs() {
	}

	public static void main(String[] args) throws IOException {
		final Path staging = Paths.get(System.getenv("ROCKUML_SMETANA_TRACE"));
		final Path output = Paths.get(args[0]);
		final int first = Integer.parseInt(args[1]);
		final int last = Integer.parseInt(args[2]);
		Files.createDirectories(output);
		for (int seed = first; seed <= last; seed++)
			System.out.print(String.format("%03d ", seed) + layOut(seed, staging, output) + "\n");
	}

	private static String layOut(int seed, Path staging, Path output) throws IOException {
		final RandomGraph graph = RandomGraph.create(seed);
		try {
			graph.layOut();
		} catch (RuntimeException | StackOverflowError e) {
			deleteAll(staging);
			return "skipped " + reason(e);
		}
		final List<Path> traces = list(staging);
		if (traces.size() != 1)
			throw new IllegalStateException("expected one trace for seed " + seed + ", found " + traces);
		final Path trace = output.resolve(String.format("%03d.trace", seed));
		Files.move(traces.get(0), trace, StandardCopyOption.REPLACE_EXISTING);
		final TreeSet<String> features = graph.features;
		if (VIRTUAL_NODE.matcher(new String(Files.readAllBytes(trace), "UTF-8")).find())
			features.add("virtual");
		final String summary = "ok " + graph.style + " nodes " + graph.nodeCount + " edges " + graph.edgeCount;
		return features.isEmpty() ? summary : summary + " " + String.join(" ", features);
	}

	/** The exception and the Smetana function that threw it, past the UNSUPPORTED helpers. */
	private static String reason(Throwable e) {
		for (StackTraceElement frame : e.getStackTrace())
			if (!frame.getClassName().equals(Macro.class.getName()))
				return e.getClass().getSimpleName() + (e.getMessage() == null ? "" : " " + e.getMessage()) + " in "
						+ frame.getClassName() + "." + frame.getMethodName();
		return e.toString();
	}

	private static List<Path> list(Path directory) throws IOException {
		try (Stream<Path> files = Files.list(directory)) {
			return files.collect(Collectors.toList());
		}
	}

	private static void deleteAll(Path directory) throws IOException {
		for (Path file : list(directory))
			Files.delete(file);
	}

	/** One graph's model, drawn from its seed, and the cgraph calls that build and lay it out. */
	private abstract static class RandomGraph {
		final Random random;
		final double density;
		final String style;
		final TreeSet<String> features = new TreeSet<>();
		Globals zz;
		ST_Agraph_s g;
		int nodeCount;
		int edgeCount;

		RandomGraph(Random random, double density, String style) {
			this.random = random;
			this.density = density;
			this.style = style;
		}

		static RandomGraph create(int seed) {
			final Random random = new Random(seed);
			final double density = Math.min(1.0, (seed - 1) / (double) FULL_DENSITY_SEED);
			final int nodes = 2 + random.nextInt(1 + (int) Math.round((MAX_NODES - 2) * density));
			final double style = random.nextDouble();
			if (style < 0.1)
				return new JsonGraph(random, density, nodes);
			if (style < 0.18)
				return new GitGraph(random, density, nodes);
			return new CucaGraph(random, density, nodes);
		}

		/** True with a probability that grows from {@code base} at seed 1 to {@code base + 0.6} at full density. */
		boolean feature(double base) {
			return random.nextDouble() < base + 0.6 * density;
		}

		boolean chance(double probability) {
			return random.nextDouble() < probability;
		}

		int between(int low, int high) {
			return low + random.nextInt(high - low + 1);
		}

		/** A text-measured length in points, as PlantUML's string bounder returns it. */
		double length(double low, double high) {
			return low + random.nextDouble() * (high - low);
		}

		void layOut() {
			zz = Globals.open();
			try {
				g = agopen(zz, new CString("g"), zz.Agdirected, null);
				build();
				final ST_GVC_s gvc = gvContext(zz);
				afterContext();
				gvLayoutJobs(zz, gvc, g);
			} finally {
				Globals.close();
			}
		}

		abstract void build();

		void afterContext() {
		}

		void set(ST_Agobj_s object, String name, String value) {
			agsafeset(zz, object, new CString(name), new CString(value), new CString(""));
		}

		void set(ST_Agobj_s object, String name, CString value) {
			agsafeset(zz, object, new CString(name), value, new CString(""));
		}

		static String inches(double points) {
			return "" + (points / 72);
		}
	}

	/** As CucaDiagramFileMakerSmetana builds class, component, use case and state diagrams. */
	private static final class CucaGraph extends RandomGraph {
		private final List<Cluster> clusters = new ArrayList<>();
		private final List<Leaf> rootLeafs = new ArrayList<>();
		private final List<Leaf> leafs = new ArrayList<>();
		private final List<Link> links = new ArrayList<>();
		private final boolean leftToRight;
		private int uid = 2;

		CucaGraph(Random random, double density, int nodes) {
			super(random, density, "cuca");
			nodeCount = nodes;
			if (nodes >= 3 && feature(0.1))
				createClusters(nodes);
			createLeafs(nodes);
			createLinks();
			leftToRight = feature(0.05);
			if (leftToRight)
				features.add("lr");
		}

		private void createClusters(int nodes) {
			final int count = between(1, Math.max(1, nodes / 3));
			final boolean nested = count > 1 && chance(0.6);
			for (int i = 0; i < count; i++) {
				final Cluster parent = nested && i > 0 && chance(0.5) ? clusters.get(random.nextInt(i)) : null;
				final Cluster cluster = new Cluster(parent, uid++);
				if (chance(0.85)) {
					cluster.labelWidth = between(20, 160);
					cluster.labelHeight = chance(0.8) ? 17 : between(26, 40);
				}
				cluster.margin = chance(0.2) ? 20 : 16;
				clusters.add(cluster);
				if (parent != null) {
					parent.children.add(cluster);
					features.add("nested");
				}
			}
			features.add("clusters");
			if (clusters.stream().anyMatch(c -> c.labelWidth > 0))
				features.add("cluster-label");
			if (clusters.stream().anyMatch(c -> c.margin == 20))
				features.add("margin20");
		}

		/** Every cluster without subclusters gets a leaf, since PlantUML draws empty packages as plain nodes. */
		private void createLeafs(int nodes) {
			final List<Cluster> mustFill = new ArrayList<>();
			for (Cluster cluster : clusters)
				if (cluster.children.isEmpty())
					mustFill.add(cluster);
			for (int i = 0; i < nodes; i++) {
				final Cluster owner;
				if (i < mustFill.size())
					owner = mustFill.get(i);
				else if (!clusters.isEmpty() && chance(0.6))
					owner = clusters.get(random.nextInt(clusters.size()));
				else
					owner = null;
				final Leaf leaf = new Leaf(String.format("sh%04d", uid++), length(20, 160),
						chance(0.6) ? 48 : length(24, 120));
				leafs.add(leaf);
				(owner == null ? rootLeafs : owner.leafs).add(leaf);
			}
		}

		private void createLinks() {
			final boolean labels = feature(0.1);
			final boolean endLabels = feature(0.05);
			final boolean flat = feature(0.1);
			final boolean longEdges = feature(0.1);
			final boolean core = !clusters.isEmpty() && feature(0.0);
			final int n = leafs.size();
			for (int i = 1; i < n; i++)
				if (chance(0.85))
					links.add(new Link(leafs.get(random.nextInt(i)), leafs.get(i)));
			for (int extra = random.nextInt(1 + (int) (n * 0.4 * density)); extra > 0; extra--) {
				final int a = random.nextInt(n);
				final int b = random.nextInt(n);
				if (a != b)
					links.add(new Link(leafs.get(Math.min(a, b)), leafs.get(Math.max(a, b))));
			}
			if (feature(0.1))
				for (int back = between(1, 3); back > 0; back--) {
					final int a = random.nextInt(n);
					final int b = random.nextInt(n);
					if (a != b)
						links.add(new Link(leafs.get(Math.max(a, b)), leafs.get(Math.min(a, b))));
				}
			if (feature(0.05))
				for (int loop = between(1, 2); loop > 0; loop--) {
					final Leaf leaf = leafs.get(random.nextInt(n));
					links.add(new Link(leaf, leaf));
					features.add("selfloop");
				}
			if (!links.isEmpty() && feature(0.05))
				for (int multi = between(1, 2); multi > 0; multi--) {
					final Link link = links.get(random.nextInt(links.size()));
					links.add(chance(0.7) ? new Link(link.tail, link.head) : new Link(link.head, link.tail));
					features.add("multi");
				}
			if (core)
				for (Link link : links) {
					if (chance(0.15))
						link.tail = clusters.get(random.nextInt(clusters.size()));
					else if (chance(0.15))
						link.head = clusters.get(random.nextInt(clusters.size()));
				}
			for (Link link : links) {
				if (flat && chance(0.3))
					link.minlen = 0;
				else if (longEdges && chance(0.25))
					link.minlen = between(2, 3);
				if (labels && chance(0.5)) {
					link.labelWidth = between(10, 130);
					link.labelHeight = chance(0.8) ? 17 : 32;
				}
				if (endLabels && chance(0.4)) {
					link.tailLabelWidth = between(8, 80);
				}
				if (endLabels && chance(0.4)) {
					link.headLabelWidth = between(8, 80);
				}
				describe(link);
			}
			edgeCount = links.size();
			if (hasCycle())
				features.add("cycle");
		}

		private void describe(Link link) {
			if (link.minlen == 0 && link.tail != link.head)
				features.add("flat");
			if (link.minlen >= 2)
				features.add("long");
			if (link.labelWidth > 0)
				features.add("label");
			if (link.labelWidth > 0 && link.minlen == 0 && link.tail != link.head)
				features.add("flat-label");
			if (link.tailLabelWidth > 0)
				features.add("taillabel");
			if (link.headLabelWidth > 0)
				features.add("headlabel");
			if (link.tail instanceof Cluster || link.head instanceof Cluster)
				features.add("core");
		}

		private boolean hasCycle() {
			final List<Endpoint> endpoints = new ArrayList<>(leafs);
			endpoints.addAll(clusters);
			final int[] state = new int[endpoints.size()];
			for (int i = 0; i < endpoints.size(); i++)
				if (state[i] == 0 && cycleFrom(i, endpoints, state))
					return true;
			return false;
		}

		private boolean cycleFrom(int i, List<Endpoint> endpoints, int[] state) {
			state[i] = 1;
			for (Link link : links)
				if (link.tail == endpoints.get(i) && link.tail != link.head) {
					final int j = endpoints.indexOf(link.head);
					if (state[j] == 1 || state[j] == 0 && cycleFrom(j, endpoints, state))
						return true;
				}
			state[i] = 2;
			return false;
		}

		@Override
		void build() {
			set(g, "margin", "16");
			for (Leaf leaf : rootLeafs)
				leaf.export(g);
			for (Cluster cluster : clusters)
				if (cluster.parent == null)
					cluster.export(g);
			for (Link link : links)
				link.export();
		}

		@Override
		void afterContext() {
			if (leftToRight)
				agsafeset(zz, g, new CString("rankdir"), new CString("LR"), new CString("LR"));
		}

		private interface Endpoint {
			ST_Agnode_s node();
		}

		private final class Leaf implements Endpoint {
			final String name;
			final double width;
			final double height;
			ST_Agnode_s node;

			Leaf(String name, double width, double height) {
				this.name = name;
				this.width = width;
				this.height = height;
			}

			void export(ST_Agraph_s graph) {
				node = agnode(zz, graph, new CString(name), true);
				set(node, "shape", "box");
				set(node, "width", inches(width));
				set(node, "height", inches(height));
			}

			@Override
			public ST_Agnode_s node() {
				return node;
			}
		}

		private final class Cluster implements Endpoint {
			final Cluster parent;
			final int uid;
			final String name;
			final List<Cluster> children = new ArrayList<>();
			final List<Leaf> leafs = new ArrayList<>();
			int labelWidth;
			int labelHeight;
			int margin;
			ST_Agraph_s graph;
			ST_Agnode_s core;

			Cluster(Cluster parent, int uid) {
				this.parent = parent;
				this.uid = uid;
				this.name = "cluster" + uid;
			}

			void export(ST_Agraph_s owner) {
				graph = agsubg(zz, owner, new CString(name), true);
				if (labelWidth > 0)
					set(graph, "label", Macro.createHackInitDimensionFromLabel(labelWidth, labelHeight));
				set(graph, "margin", "" + margin);
				for (Leaf leaf : leafs)
					leaf.export(graph);
				for (Cluster child : children)
					child.export(graph);
			}

			/** The group's core node, created on first use like CucaDiagramFileMakerSmetana.getCoreFromGroup. */
			@Override
			public ST_Agnode_s node() {
				if (core == null) {
					core = agnode(zz, graph, new CString(String.format("zent%04d", uid)), true);
					set(core, "shape", "box");
					set(core, "width", "0.1");
					set(core, "height", "0.1");
				}
				return core;
			}
		}

		private final class Link {
			Endpoint tail;
			Endpoint head;
			int minlen = 1;
			int labelWidth;
			int labelHeight;
			int tailLabelWidth;
			int headLabelWidth;

			Link(Endpoint tail, Endpoint head) {
				this.tail = tail;
				this.head = head;
			}

			void export() {
				final ST_Agnode_s node1 = tail.node();
				final ST_Agnode_s node2 = head.node();
				final ST_Agobj_s e = agedge(zz, g, node1, node2, null, true);
				set(e, "arrowtail", "none");
				set(e, "arrowhead", "none");
				set(e, "minlen", "" + minlen);
				if (labelWidth > 0)
					set(e, "label", Macro.createHackInitDimensionFromLabel(labelWidth, labelHeight));
				if (tailLabelWidth > 0)
					set(e, "taillabel", Macro.createHackInitDimensionFromLabel(tailLabelWidth, 15));
				if (headLabelWidth > 0)
					set(e, "headlabel", Macro.createHackInitDimensionFromLabel(headLabelWidth, 15));
			}
		}
	}

	/** As SmetanaForGit lays out a git log: boxes, each commit linked to the commits below it. */
	private static final class GitGraph extends RandomGraph {
		private final double[][] sizes;
		private final List<List<Integer>> downs = new ArrayList<>();

		GitGraph(Random random, double density, int nodes) {
			super(random, density, "git");
			nodeCount = nodes;
			sizes = new double[nodes][];
			for (int i = 0; i < nodes; i++) {
				sizes[i] = new double[] { length(40, 140), length(24, 40) };
				final List<Integer> down = new ArrayList<>();
				if (i > 0)
					down.add(i - 1 - random.nextInt(Math.min(i, 3)));
				if (i > 1 && chance(0.2 + 0.2 * density)) {
					final int merge = random.nextInt(i);
					if (!down.contains(merge))
						down.add(merge);
				}
				edgeCount += down.size();
				downs.add(down);
			}
		}

		@Override
		void build() {
			set(g, "ranksep", "0.35");
			final List<ST_Agnode_s> nodes = new ArrayList<>();
			for (int i = 0; i < nodeCount; i++) {
				final ST_Agnode_s node = agnode(zz, g, new CString("N" + i), true);
				set(node, "shape", "box");
				set(node, "height", inches(sizes[i][1]));
				set(node, "width", inches(sizes[i][0]));
				nodes.add(node);
			}
			for (int i = 0; i < nodeCount; i++)
				for (int down : downs.get(i)) {
					final ST_Agobj_s e = agedge(zz, g, nodes.get(i), nodes.get(down), null, true);
					set(e, "arrowsize", ".7");
					set(e, "arrowtail", "none");
					set(e, "arrowhead", "normal");
				}
		}
	}

	/** As SmetanaForJson lays out a JSON or YAML tree: record nodes, each child linked from its row's port. */
	private static final class JsonGraph extends RandomGraph {
		private int budget;
		private int num;

		JsonGraph(Random random, double density, int nodes) {
			super(random, density, "json");
			budget = nodes;
			features.add("record");
		}

		@Override
		void build() {
			manageOneNode();
		}

		private ST_Agnode_s manageOneNode() {
			budget--;
			nodeCount++;
			final boolean isArray = chance(0.3);
			// An empty root would leave the label attribute undeclared, and Smetana cannot parse the default "\N"
			// as a record label.
			final boolean root = num == 0;
			final int rows = !root && chance(0.05) ? 0 : between(1, 7);
			final double[] lineHeights = new double[rows];
			for (int i = 0; i < rows; i++)
				lineHeights[i] = chance(0.9) ? 18.0 : 36.0;
			final double colA = length(20, 120);
			final double colB = isArray || rows == 0 ? 0 : length(20, 120);
			double height = rows == 0 ? 15 : 0;
			for (double h : lineHeights)
				height += h;
			final ST_Agnode_s node = createNode(rows == 0 ? 0 : colA + colB, height, isArray, colA, colB,
					lineHeights);
			boolean parent = false;
			for (int i = 0; i < rows; i++)
				// The root always gets a child, so that every graph has an edge.
				if (budget > 0 && (chance(0.6) || root && !parent && i == rows - 1)) {
					parent = true;
					final ST_Agnode_s child = manageOneNode();
					final ST_Agobj_s edge = agedge(zz, g, node, child, null, true);
					edgeCount++;
					set(edge, "arrowsize", ".75");
					set(edge, "arrowtail", "none");
					set(edge, "arrowhead", "normal");
					set(edge, "tailport", "P" + i);
				}
			return node;
		}

		private ST_Agnode_s createNode(double width, double height, boolean isArray, double colA, double colB,
				double[] lineHeights) {
			final ST_Agnode_s node = agnode(zz, g, new CString("N" + num++), true);
			set(node, "shape", "record");
			// Swapped as in SmetanaForJson, which draws the layout transposed.
			set(node, "height", inches(width));
			set(node, "width", inches(height));
			if (lineHeights.length > 0)
				set(node, "label", isArray ? arrayLabel(colA - 8, lineHeights) : mapLabel(colA - 8, colB - 8,
						lineHeights));
			return node;
		}

		private static String arrayLabel(double widthA, double[] lineHeights) {
			final StringBuilder sb = new StringBuilder();
			for (int i = 0; i < lineHeights.length; i++) {
				sb.append("<P" + i + ">");
				sb.append("_dim_" + lineHeights[i] + "_" + widthA + "_");
				if (i < lineHeights.length - 1)
					sb.append("|");
			}
			return sb.toString();
		}

		private static String mapLabel(double widthA, double widthB, double[] lineHeights) {
			double height = 0;
			for (double h : lineHeights)
				height += h;
			final StringBuilder sb = new StringBuilder("{_dim_" + height + "_" + widthA + "_|{");
			for (int i = 0; i < lineHeights.length; i++) {
				sb.append("<P" + i + ">");
				sb.append("_dim_" + lineHeights[i] + "_" + widthB + "_");
				if (i < lineHeights.length - 1)
					sb.append("|");
			}
			return sb.append("}}").toString();
		}
	}
}
