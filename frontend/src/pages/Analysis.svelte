<script>
  import { onMount } from 'svelte'
  import Card from '../components/Card.svelte'
  import { getLatencyDistribution, getAnomalies, getServices } from '../api.js'

  let latencyDist = null
  let anomalies = []
  let services = []
  let selectedService = ''
  let loading = true

  onMount(async () => {
    await loadData()
  })

  async function loadData() {
    loading = true
    try {
      const params = {}
      if (selectedService) params.service = selectedService

      const [distRes, anomRes, svcRes] = await Promise.all([
        getLatencyDistribution(params),
        getAnomalies({ page_size: 10 }),
        getServices(),
      ])
      latencyDist = distRes.data
      anomalies = anomRes.data
      services = svcRes.data
    } catch (e) {
      console.error('Failed to load analysis data:', e)
    } finally {
      loading = false
    }
  }

  function handleServiceChange() {
    loadData()
  }
</script>

<div class="analysis-page">
  <div class="page-header">
    <h1>性能分析</h1>
    <p class="subtitle">多维度分析服务性能指标和异常</p>
  </div>

  <div class="filter-bar">
    <div class="filter-item">
      <label>服务筛选</label>
      <select bind:value={selectedService} on:change={handleServiceChange}>
        <option value="">全部服务</option>
        {#each services as svc}
          <option value={svc.service_name}>{svc.service_name}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else}
    <div class="metrics-row">
      <Card title="延迟分位数">
        <div class="quantiles">
          <div class="quantile-item">
            <span class="quantile-label">P50</span>
            <span class="quantile-value">{latencyDist?.p50?.toFixed(1) || 0}ms</span>
          </div>
          <div class="quantile-item">
            <span class="quantile-label">P95</span>
            <span class="quantile-value">{latencyDist?.p95?.toFixed(1) || 0}ms</span>
          </div>
          <div class="quantile-item">
            <span class="quantile-label">P99</span>
            <span class="quantile-value">{latencyDist?.p99?.toFixed(1) || 0}ms</span>
          </div>
        </div>
      </Card>
    </div>

    <div class="content-grid">
      <Card title="延迟分布直方图">
        <div class="histogram">
          {#if latencyDist?.buckets?.length}
            {#each latencyDist.buckets as bucket}
              <div class="histogram-bar">
                <div class="bar-label">{bucket.min_ms}-{bucket.max_ms}ms</div>
                <div class="bar-container">
                  <div 
                    class="bar-fill" 
                    style="width: {Math.min(bucket.count / Math.max(...latencyDist.buckets.map(b => b.count), 1) * 100, 100)}%;"
                  />
                </div>
                <div class="bar-count">{bucket.count}</div>
              </div>
            {/each}
          {:else}
            <div class="empty">暂无数据</div>
          {/if}
        </div>
      </Card>

      <Card title="最近异常检测">
        <div class="anomaly-list">
          {#if anomalies.length === 0}
            <div class="empty">暂无异常</div>
          {:else}
            {#each anomalies as anomaly}
              <div class="anomaly-card">
                <div class="anomaly-top">
                  <span class={`severity severity-${anomaly.severity}`}>{anomaly.severity}</span>
                  <span class="anomaly-type">{anomaly.anomaly_type}</span>
                </div>
                <div class="anomaly-service">
                  {anomaly.root_service || 'Unknown'} / {anomaly.root_operation || 'Unknown'}
                </div>
                <div class="anomaly-time">
                  {new Date(anomaly.detected_at).toLocaleString()}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </Card>
    </div>
  {/if}
</div>

<style>
  .analysis-page {
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
  .filter-bar {
    display: flex;
    gap: 16px;
    align-items: flex-end;
  }
  .filter-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .filter-item label {
    font-size: 13px;
    color: #94a3b8;
    font-weight: 500;
  }
  .filter-item select {
    padding: 8px 12px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
    min-width: 200px;
  }
  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .metrics-row {
    display: grid;
    grid-template-columns: 1fr;
  }
  .quantiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  .quantile-item {
    text-align: center;
    padding: 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .quantile-label {
    display: block;
    font-size: 13px;
    color: #94a3b8;
    margin-bottom: 8px;
  }
  .quantile-value {
    font-size: 28px;
    font-weight: 700;
    color: #3b82f6;
  }
  .content-grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 16px;
  }
  .histogram {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .histogram-bar {
    display: grid;
    grid-template-columns: 100px 1fr 60px;
    gap: 12px;
    align-items: center;
  }
  .bar-label {
    font-size: 12px;
    color: #94a3b8;
    text-align: right;
  }
  .bar-container {
    height: 20px;
    background: #0f172a;
    border-radius: 4px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b82f6, #60a5fa);
    border-radius: 4px;
  }
  .bar-count {
    font-size: 12px;
    color: #e2e8f0;
    font-weight: 500;
  }
  .anomaly-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 400px;
    overflow-y: auto;
  }
  .anomaly-card {
    padding: 12px;
    background: #0f172a;
    border-radius: 8px;
    border-left: 3px solid #ef4444;
  }
  .anomaly-top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .severity {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .severity-critical { background: rgba(239, 68, 68, 0.2); color: #ef4444; }
  .severity-warning { background: rgba(234, 179, 8, 0.2); color: #eab308; }
  .severity-info { background: rgba(59, 130, 246, 0.2); color: #3b82f6; }
  .anomaly-type {
    font-size: 12px;
    color: #94a3b8;
  }
  .anomaly-service {
    font-size: 14px;
    color: #e2e8f0;
    font-weight: 500;
    margin-bottom: 4px;
  }
  .anomaly-time {
    font-size: 12px;
    color: #64748b;
  }
  @media (max-width: 1024px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
