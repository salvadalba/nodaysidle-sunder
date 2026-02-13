import { useEffect, useRef, useCallback, useState } from "react";
import { useAppState } from "./AppShell";
import { useGraphData } from "../hooks/useGraphData";
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";
import { zoom, zoomIdentity, type ZoomBehavior } from "d3-zoom";
import { select } from "d3-selection";
import { drag } from "d3-drag";

// --- Types ---

interface GraphNode extends SimulationNodeDatum {
  id: string;
  title: string;
  cluster: number;
}

interface GraphLink extends SimulationLinkDatum<GraphNode> {
  weight: number;
}

// Cluster color palette — deep cartography aesthetic
const CLUSTER_COLORS = [
  "var(--color-amber)",
  "var(--color-violet)",
  "var(--color-teal)",
  "#e06c75",
  "#c678dd",
  "#98c379",
  "#61afef",
  "#d19a66",
];

function clusterColor(cluster: number): string {
  return CLUSTER_COLORS[cluster % CLUSTER_COLORS.length];
}

// --- Component ---

export function GraphCanvas() {
  const { state, dispatch } = useAppState();
  const { data, isLoading } = useGraphData(
    state.activeNoteId && state.activeNoteId !== "new" ? state.activeNoteId : undefined,
  );
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simulationRef = useRef<ReturnType<typeof forceSimulation<GraphNode>> | null>(null);
  const zoomRef = useRef<ZoomBehavior<SVGSVGElement, unknown> | null>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });

  // Track container size
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setDimensions({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Navigate to note on click
  const handleNodeClick = useCallback(
    (nodeId: string) => {
      dispatch({ type: "SET_ACTIVE_NOTE", id: nodeId });
    },
    [dispatch],
  );

  // Build and run simulation
  useEffect(() => {
    if (!data || !svgRef.current) return;

    const { width, height } = dimensions;

    // Clone data for d3 mutation
    const nodes: GraphNode[] = data.nodes.map((n) => ({
      id: n.id,
      title: n.title,
      cluster: n.cluster,
      x: n.x || width / 2 + (Math.random() - 0.5) * 200,
      y: n.y || height / 2 + (Math.random() - 0.5) * 200,
    }));

    const nodeMap = new Map(nodes.map((n) => [n.id, n]));

    const links: GraphLink[] = data.edges
      .filter((e) => nodeMap.has(e.source) && nodeMap.has(e.target))
      .map((e) => ({
        source: e.source,
        target: e.target,
        weight: e.weight,
      }));

    // Kill old simulation
    simulationRef.current?.stop();

    const sim = forceSimulation<GraphNode>(nodes)
      .force(
        "link",
        forceLink<GraphNode, GraphLink>(links)
          .id((d) => d.id)
          .distance((d) => 120 * (1 - d.weight))
          .strength((d) => d.weight * 0.8),
      )
      .force("charge", forceManyBody().strength(-200))
      .force("center", forceCenter(width / 2, height / 2))
      .force("collide", forceCollide<GraphNode>().radius(30))
      .alphaDecay(0.02);

    simulationRef.current = sim;

    const svg = select(svgRef.current);

    // Setup zoom
    const zoomBehavior = zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 4])
      .on("zoom", (event) => {
        svg.select<SVGGElement>("g.graph-root").attr("transform", event.transform);
      });

    zoomRef.current = zoomBehavior;
    svg.call(zoomBehavior);

    // Get or create root group
    let g = svg.select<SVGGElement>("g.graph-root");
    if (g.empty()) {
      g = svg.append("g").attr("class", "graph-root");
    }
    g.selectAll("*").remove();

    // Defs for glassmorphic blur
    let defs = svg.select<SVGDefsElement>("defs");
    if (defs.empty()) {
      defs = svg.append("defs");
    }
    defs.selectAll("*").remove();
    const filter = defs.append("filter").attr("id", "node-glow");
    filter.append("feGaussianBlur").attr("stdDeviation", "3").attr("result", "blur");
    filter.append("feMerge").call((m) => {
      m.append("feMergeNode").attr("in", "blur");
      m.append("feMergeNode").attr("in", "SourceGraphic");
    });

    // Draw edges
    const linkSelection = g
      .selectAll<SVGLineElement, GraphLink>("line.graph-edge")
      .data(links)
      .join("line")
      .attr("class", "graph-edge")
      .attr("stroke", "var(--color-ink-ghost)")
      .attr("stroke-width", (d) => Math.max(0.5, d.weight * 2))
      .attr("stroke-opacity", (d) => 0.15 + d.weight * 0.4);

    // Draw nodes
    const nodeGroup = g
      .selectAll<SVGGElement, GraphNode>("g.graph-node")
      .data(nodes, (d) => d.id)
      .join("g")
      .attr("class", "graph-node")
      .style("cursor", "pointer");

    // Node circle — glassmorphic style
    nodeGroup
      .append("circle")
      .attr("r", (d) => (d.id === state.activeNoteId ? 14 : 10))
      .attr("fill", (d) => {
        const c = clusterColor(d.cluster);
        return d.id === state.activeNoteId ? c : `color-mix(in oklch, ${c} 25%, var(--color-surface-2))`;
      })
      .attr("stroke", (d) => clusterColor(d.cluster))
      .attr("stroke-width", (d) => (d.id === state.activeNoteId ? 2 : 1))
      .attr("stroke-opacity", (d) => (d.id === state.activeNoteId ? 1 : 0.5))
      .attr("filter", (d) => (d.id === state.activeNoteId ? "url(#node-glow)" : "none"));

    // Node label
    nodeGroup
      .append("text")
      .text((d) => d.title.length > 20 ? d.title.slice(0, 20) + "..." : d.title)
      .attr("dy", 24)
      .attr("text-anchor", "middle")
      .attr("fill", "var(--color-ink-tertiary)")
      .attr("font-size", "10px")
      .attr("font-family", "var(--font-body)")
      .attr("font-weight", "400")
      .attr("pointer-events", "none");

    // Click handler
    nodeGroup.on("click", (_event, d) => {
      handleNodeClick(d.id);
    });

    // Drag behavior
    const dragBehavior = drag<SVGGElement, GraphNode>()
      .on("start", (event, d) => {
        if (!event.active) sim.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on("drag", (event, d) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on("end", (event, d) => {
        if (!event.active) sim.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      });

    nodeGroup.call(dragBehavior);

    // Hover effects
    nodeGroup
      .on("mouseenter", function () {
        select(this).select("circle").attr("stroke-opacity", 1);
        select(this).select("text").attr("fill", "var(--color-ink-secondary)");
      })
      .on("mouseleave", function (_, d) {
        select(this)
          .select("circle")
          .attr("stroke-opacity", d.id === state.activeNoteId ? 1 : 0.5);
        select(this).select("text").attr("fill", "var(--color-ink-tertiary)");
      });

    // Simulation tick
    sim.on("tick", () => {
      linkSelection
        .attr("x1", (d) => (d.source as GraphNode).x!)
        .attr("y1", (d) => (d.source as GraphNode).y!)
        .attr("x2", (d) => (d.target as GraphNode).x!)
        .attr("y2", (d) => (d.target as GraphNode).y!);

      nodeGroup.attr("transform", (d) => `translate(${d.x},${d.y})`);
    });

    // Fit to view
    svg.call(zoomBehavior.transform, zoomIdentity);

    return () => {
      sim.stop();
    };
  }, [data, dimensions, state.activeNoteId, handleNodeClick]);

  if (isLoading && !data) {
    return (
      <div className="flex h-full items-center justify-center" style={{ background: "var(--color-void)" }}>
        <span className="text-[12px]" style={{ color: "var(--color-ink-ghost)" }}>
          Building graph...
        </span>
      </div>
    );
  }

  if (!data || data.nodes.length === 0) {
    return (
      <div className="flex h-full items-center justify-center" style={{ background: "var(--color-void)" }}>
        <div className="flex flex-col items-center gap-2">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" style={{ opacity: 0.3 }}>
            <circle cx="8" cy="8" r="3" stroke="var(--color-ink-ghost)" strokeWidth="1.5" />
            <circle cx="16" cy="10" r="3" stroke="var(--color-ink-ghost)" strokeWidth="1.5" />
            <circle cx="10" cy="18" r="3" stroke="var(--color-ink-ghost)" strokeWidth="1.5" />
            <line x1="10.5" y1="9.5" x2="13.5" y2="8.5" stroke="var(--color-ink-ghost)" strokeWidth="1" />
            <line x1="9" y1="10.5" x2="9.5" y2="15.5" stroke="var(--color-ink-ghost)" strokeWidth="1" />
          </svg>
          <span className="text-[11px]" style={{ color: "var(--color-ink-ghost)" }}>
            No connections yet
          </span>
          <span className="text-[10px]" style={{ color: "var(--color-ink-ghost)", opacity: 0.6 }}>
            Create more notes to see the graph
          </span>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="relative h-full"
      style={{
        background: "var(--color-void)",
        animation: "fade-in 0.3s ease-out",
      }}
    >
      <svg
        ref={svgRef}
        width={dimensions.width}
        height={dimensions.height}
        style={{ display: "block" }}
      />

      {/* Legend */}
      <div
        className="absolute bottom-3 left-3 flex items-center gap-3 rounded-lg px-3 py-1.5"
        style={{
          background: "oklch(0.12 0.015 270 / 0.8)",
          backdropFilter: "blur(8px)",
          border: "1px solid var(--color-border-subtle)",
        }}
      >
        <span className="text-[9px] font-500 uppercase tracking-wider" style={{ color: "var(--color-ink-ghost)" }}>
          {data.nodes.length} nodes · {data.edges.length} edges
        </span>
      </div>

      {/* Controls hint */}
      <div
        className="absolute bottom-3 right-3 flex items-center gap-2 rounded-lg px-3 py-1.5"
        style={{
          background: "oklch(0.12 0.015 270 / 0.8)",
          backdropFilter: "blur(8px)",
          border: "1px solid var(--color-border-subtle)",
        }}
      >
        <span className="text-[9px] font-300" style={{ color: "var(--color-ink-ghost)" }}>
          Scroll to zoom · Drag nodes · Click to navigate
        </span>
      </div>
    </div>
  );
}
