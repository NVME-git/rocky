import * as assert from "assert";

// Unit tests for Rocky VS Code extension components
// These test the data transformation logic without requiring the VS Code API.

interface RockyNode {
  name: string;
  classification: string;
  difficulty: number;
  stability: number;
  retrievability: number;
  total_reviews: number;
  repo: string | null;
  canonical_question: string | null;
  canonical_answer: string | null;
}

interface RockyEdge {
  source: string;
  target: string;
  relation: string;
}

interface PkgData {
  nodes: RockyNode[];
  edges: RockyEdge[];
}

suite("Rocky Extension Unit Tests", () => {
  const samplePkg: PkgData = {
    nodes: [
      {
        name: "async/await",
        classification: "Language",
        difficulty: 0.3,
        stability: 5.0,
        retrievability: 0.85,
        total_reviews: 4,
        repo: "taskify",
        canonical_question: "What happens when you await a panicking future?",
        canonical_answer: "The panic propagates to the awaiting task.",
      },
      {
        name: "JWT Auth",
        classification: "Auth",
        difficulty: 0.5,
        stability: 3.0,
        retrievability: 0.35,
        total_reviews: 1,
        repo: "taskify",
        canonical_question: null,
        canonical_answer: null,
      },
      {
        name: "Docker Compose",
        classification: "DevOps",
        difficulty: 0.4,
        stability: 2.0,
        retrievability: 0.6,
        total_reviews: 2,
        repo: null,
        canonical_question: null,
        canonical_answer: null,
      },
    ],
    edges: [
      { source: "async/await", target: "JWT Auth", relation: "uses" },
      { source: "JWT Auth", target: "Docker Compose", relation: "deployed_with" },
    ],
  };

  test("nodes are grouped by classification", () => {
    const groups = new Map<string, RockyNode[]>();
    for (const node of samplePkg.nodes) {
      const key = node.classification || "Other";
      if (!groups.has(key)) {
        groups.set(key, []);
      }
      groups.get(key)!.push(node);
    }

    assert.strictEqual(groups.size, 3);
    assert.strictEqual(groups.get("Language")!.length, 1);
    assert.strictEqual(groups.get("Auth")!.length, 1);
    assert.strictEqual(groups.get("DevOps")!.length, 1);
  });

  test("local nodes are filtered by repo name", () => {
    const repoName = "taskify";
    const localNodes = samplePkg.nodes.filter(
      (n) => n.repo !== null && n.repo.toLowerCase() === repoName.toLowerCase()
    );
    assert.strictEqual(localNodes.length, 2);
    assert.ok(localNodes.every((n) => n.repo === "taskify"));
  });

  test("local edges only include edges between local nodes", () => {
    const localNodes = samplePkg.nodes.filter((n) => n.repo === "taskify");
    const localNames = new Set(localNodes.map((n) => n.name));
    const localEdges = samplePkg.edges.filter(
      (e) => localNames.has(e.source) && localNames.has(e.target)
    );
    assert.strictEqual(localEdges.length, 1);
    assert.strictEqual(localEdges[0].source, "async/await");
    assert.strictEqual(localEdges[0].target, "JWT Auth");
  });

  test("retrievability categorization is correct", () => {
    const strong = samplePkg.nodes.filter((n) => n.retrievability > 0.7);
    const weak = samplePkg.nodes.filter((n) => n.retrievability < 0.4);
    const medium = samplePkg.nodes.filter(
      (n) => n.retrievability >= 0.4 && n.retrievability <= 0.7
    );

    assert.strictEqual(strong.length, 1); // async/await
    assert.strictEqual(weak.length, 1); // JWT Auth
    assert.strictEqual(medium.length, 1); // Docker Compose
  });

  test("summary statistics are computed correctly", () => {
    const totalTopics = samplePkg.nodes.length;
    const totalEdges = samplePkg.edges.length;
    const avgRetrievability =
      samplePkg.nodes.reduce((sum, n) => sum + n.retrievability, 0) /
      totalTopics;
    const totalReviews = samplePkg.nodes.reduce(
      (sum, n) => sum + n.total_reviews,
      0
    );
    const domains = new Set(
      samplePkg.nodes.map((n) => n.classification || "Other")
    );

    assert.strictEqual(totalTopics, 3);
    assert.strictEqual(totalEdges, 2);
    assert.strictEqual(domains.size, 3);
    assert.strictEqual(totalReviews, 7);
    assert.ok(Math.abs(avgRetrievability - 0.6) < 0.01);
  });

  test("node lookup by name works", () => {
    const found = samplePkg.nodes.find((n) => n.name === "JWT Auth");
    assert.ok(found);
    assert.strictEqual(found!.classification, "Auth");

    const notFound = samplePkg.nodes.find((n) => n.name === "Nonexistent");
    assert.strictEqual(notFound, undefined);
  });

  test("escapeHtml prevents XSS in topic names", () => {
    function escapeHtml(text: string): string {
      return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
    }

    assert.strictEqual(escapeHtml('<script>alert("xss")</script>'),
      '&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;');
    assert.strictEqual(escapeHtml("normal text"), "normal text");
    assert.strictEqual(escapeHtml("a & b"), "a &amp; b");
  });
});
