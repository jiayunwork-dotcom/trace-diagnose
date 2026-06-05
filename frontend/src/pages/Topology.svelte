<script>
  import { onMount, onDestroy } from 'svelte'
  import Card from '../components/Card.svelte'
  import { getTopology } from '../api.js'

  let topology = { nodes: [], edges: [] }
  let loading = true
  let selectedNode = null
  let container

  onMount(async () => {
    try {
      const res = await getTopology()
      topology = res.data
      renderGraph()
    } catch (e) {
      console.error('Failed to load topology:', e)
    } finally {
      loading = false
    }
  })

  function renderGraph() {
    if (!container || topology.nodes.length === 0) return

    container.innerHTML = ''
    const width = container.clientWidth
    const height = 600
    const padding = 40

    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    svg.setAttribute('width', width)
    svg.setAttribute('height', height)
    svg.style.cursor = 'grab'

    const g = document.createElementNS('http://www.w3.org/2000/svg', 'g')
    svg.appendChild(g)

    const nodePositions = {}
    const angleStep = (2 * Math.PI) / Math.max(topology.nodes.length, 1)
    const centerX = width / 2
    const centerY = height / 2
    const radius = Math.min(width, height) / 2 - padding * 2

    topology.nodes.forEach((node, i) => {
      const angle = i * angleStep - Math.PI / 2
      nodePositions[node.id] = {
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      }
    })

    topology.edges.forEach(edge => {
      const source = nodePositions[edge.source]
      const target = nodePositions[edge.target]
      if (!source || !target) return

      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line')
      line.setAttribute('x1', source.x)
      line.setAttribute('y1', source.y)
      line.setAttribute('x2', target.x)
      line.setAttribute('y2', target.y)
      line.setAttribute('stroke', '#475569')
      line.setAttribute('stroke-width', Math.min(Math.sqrt(edge.call_count) * 0.5, 8))
      line.setAttribute('stroke-opacity', '0.6')
      g.appendChild(line)
    })

    topology.nodes.forEach(node => {
      const pos = nodePositions[node.id]
      if (!pos) return

      const nodeSize = Math.min(20 + Math.sqrt(node.qps * 100) * 2, 60)
      const errorColor = node.error_rate > 0.05 ? '#ef4444' : node.error_rate > 0.01 ? '#eab308' : '#22c55e'

      const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle')
      circle.setAttribute('cx', pos.x)
      circle.setAttribute('cy', pos.y)
      circle.setAttribute('r', nodeSize)
      circle.setAttribute('fill', errorColor)
      circle.setAttribute('fill-opacity', '0.2')
      circle.setAttribute('stroke', errorColor)
      circle.setAttribute('stroke-width', '2')
      circle.style.cursor = 'pointer'
      circle.addEventListener('click', () => {
        selectedNode = node
      })
      g.appendChild(circle)

      const text = document.createElementNS('http://www.w3.org/2000/svg', 'text')
      text.setAttribute('x', pos.x)
      text.setAttribute('y', pos.y + nodeSize + 20)
      text.setAttribute('text-anchor', 'middle')
      text.setAttribute('fill', '#e2e8f0')
      text.setAttribute('font-size', '12')
      text.textContent = node.service_name
      g.appendChild(text)
    })

    container.appendChild(svg)
  }
</script>

<div class="topology-page">
  <div class="page-header">
    <h1>服务拓扑图</h1>
    <p class="subtitle">可视化展示服务间的依赖关系和调用情况</p>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else}
    <div class="topology-layout">
      <Card title="服务依赖图" subtitle="节点大小=QPS · 颜色=错误率 · 边粗细=调用频次">
        <div bind:this={container} class="graph-container" />
      </Card>

      {#if selectedNode}
        <Card title={`服务详情: ${selectedNode.service_name}`}>
          <div class="detail-panel">
            <div class="detail-row">
              <span class="label">QPS</span>
              <span class="value">{selectedNode.qps.toFixed(2)}</span>
            </div>
            <div class="detail-row">
              <span class="label">错误率</span>
              <span class="value" class:error={selectedNode.error_rate > 0.05}>
                {(selectedNode.error_rate * 100).toFixed(2)}%
              </span>
            </div>
            <div class="detail-row">
              <span class="label">平均延迟</span>
              <span class="value">{selectedNode.avg_duration_ms.toFixed(0)}ms</span>
            </div>
            <button class="close-btn" on:click={() => selectedNode = null}>关闭</button>
          </div>
        </Card>
      {:else}
        <Card title="图例说明">
          <div class="legend">
            <div class="legend-item">
              <div class="legend-dot green"></div>
              <span>错误率 < 1%</span>
            </div>
            <div class="legend-item">
              <div class="legend-dot yellow"></div>
              <span>错误率 1-5%</span>
            </div>
            <div class="legend-item">
              <div class="legend-dot red"></div>
              <span>错误率 > 5%</span>
            </div>
            <hr class="legend-divider" />
            <div class="legend-item">
              <span class="legend-line thin"></span>
              <span>调用频次低</span>
            </div>
            <div class="legend-item">
              <span class="legend-line thick"></span>
              <span>调用频次高</span>
            </div>
          </div>
        </Card>
      {/if}
    </div>
  {/if}
</div>

<style>
  .topology-page {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .page-header h1 {
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
    margin: 0 0 8px 0;
  }
  .subtitle {
    color: #94a3b8;
    font-size: 14px;
    margin: 0;
  }
  .loading {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .topology-layout {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 16px;
  }
  .graph-container {
    width: 100%;
    height: 600px;
    background: #0f172a;
    border-radius: 8px;
    overflow: hidden;
  }
  .detail-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .detail-row {
    display: flex;
    justify-content: space-between;
    padding: 12px;
    background: #0f172a;
    border-radius: 8px;
  }
  .label {
    color: #94a3b8;
    font-size: 14px;
  }
  .value {
    font-weight: 600;
    color: #f1f5f9;
  }
  .value.error {
    color: #ef4444;
  }
  .close-btn {
    padding: 8px 16px;
    background: #334155;
    border: none;
    border-radius: 6px;
    color: #e2e8f0;
    cursor: pointer;
    font-size: 14px;
  }
  .close-btn:hover {
    background: #475569;
  }
  .legend {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14px;
    color: #e2e8f0;
  }
  .legend-dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
  }
  .legend-dot.green { background: #22c55e; }
  .legend-dot.yellow { background: #eab308; }
  .legend-dot.red { background: #ef4444; }
  .legend-line {
    width: 32px;
    height: 0;
    border-top: 2px solid #475569;
  }
  .legend-line.thick {
    border-top-width: 6px;
  }
  .legend-divider {
    border: none;
    border-top: 1px solid #334155;
    margin: 8px 0;
  }
  @media (max-width: 1024px) {
    .topology-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
