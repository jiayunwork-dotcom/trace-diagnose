<script>
  import { onMount } from 'svelte'
  import StatCard from '../components/StatCard.svelte'
  import Card from '../components/Card.svelte'
  import { getServices, getTraces, getAnomalies } from '../api.js'

  let services = []
  let recentTraces = []
  let anomalies = []
  let loading = true

  onMount(async () => {
    try {
      const [svcRes, tracesRes, anomRes] = await Promise.all([
        getServices(),
        getTraces({ page_size: 5 }),
        getAnomalies({ page_size: 5 }),
      ])
      services = svcRes.data
      recentTraces = tracesRes.data?.data || []
      anomalies = anomRes.data
    } catch (e) {
      console.error('Failed to load dashboard data:', e)
    } finally {
      loading = false
    }
  })

  const totalSpans = services.reduce((sum, s) => sum + (s.span_count || 0), 0)
  const totalErrors = services.reduce((sum, s) => sum + (s.error_rate || 0) * (s.span_count || 0), 0)
  const avgErrorRate = services.length > 0 ? (totalErrors / totalSpans * 100).toFixed(2) : '0'
</script>

<div class="dashboard">
  <div class="page-header">
    <h1>控制台概览</h1>
    <p class="subtitle">实时监控您的微服务链路健康状态</p>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else}
    <div class="stats-grid">
      <StatCard label="服务总数" value={services.length.toString()} icon="🔧" color="blue" />
      <StatCard label="今日Trace数" value={recentTraces.length?.toString() || '0'} icon="📋" color="green" />
      <StatCard label="总Span数" value={totalSpans.toString()} icon="📊" color="purple" />
      <StatCard label="平均错误率" value={`${avgErrorRate}%`} icon="⚠️" color={parseFloat(avgErrorRate) > 5 ? 'red' : 'yellow'} />
    </div>

    <div class="content-grid">
      <Card title="服务列表" subtitle="所有已注册服务的健康状态">
        <div class="service-list">
          {#if services.length === 0}
            <div class="empty">暂无服务数据，请先导入Trace数据</div>
          {:else}
            {#each services as service}
              <div class="service-item">
                <div class="service-info">
                  <span class="service-name">{service.service_name}</span>
                  <span class="service-stats">
                    {service.span_count} spans · {service.trace_count} traces
                  </span>
                </div>
                <div class="service-metrics">
                  <span class="metric">
                    <span class="metric-label">P95延迟</span>
                    <span class="metric-value">{service.avg_duration_ms?.toFixed(0)}ms</span>
                  </span>
                  <span class="error-badge" class:error={service.error_rate > 0.05}>
                    {(service.error_rate * 100).toFixed(2)}% 错误
                  </span>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </Card>

      <Card title="最近异常" subtitle="检测到的性能异常和错误">
        <div class="anomaly-list">
          {#if anomalies.length === 0}
            <div class="empty">暂无异常记录</div>
          {:else}
            {#each anomalies as anomaly}
              <div class="anomaly-item">
                <div class="anomaly-header">
                  <span class={`severity severity-${anomaly.severity}`}>{anomaly.severity}</span>
                  <span class="anomaly-type">{anomaly.anomaly_type}</span>
                </div>
                <div class="anomaly-detail">
                  {anomaly.root_service} / {anomaly.root_operation}
                </div>
                <div class="anomaly-trace">Trace: {anomaly.trace_id.slice(0, 16)}...</div>
              </div>
            {/each}
          {/if}
        </div>
      </Card>
    </div>

    <Card title="最近Trace" subtitle="最新导入的调用链路">
      <div class="trace-list">
        {#if recentTraces.length === 0}
          <div class="empty">暂无Trace数据</div>
        {:else}
          <table class="trace-table">
            <thead>
              <tr>
                <th>Trace ID</th>
                <th>Span数</th>
                <th>服务数</th>
                <th>持续时间</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              {#each recentTraces as trace}
                <tr>
                  <td class="trace-id">{trace.trace_id.slice(0, 20)}...</td>
                  <td>{trace.span_count}</td>
                  <td>{trace.service_count}</td>
                  <td>{trace.duration_ms}ms</td>
                  <td>
                    <span class={`status-badge ${trace.has_errors ? 'error' : 'ok'}`}>
                      {trace.has_errors ? '有错误' : '正常'}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </Card>
  {/if}
</div>

<style>
  .dashboard {
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
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 16px;
  }
  .content-grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 16px;
  }
  .service-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .service-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .service-name {
    font-weight: 600;
    color: #f1f5f9;
  }
  .service-stats {
    font-size: 12px;
    color: #64748b;
    margin-left: 12px;
  }
  .service-metrics {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .metric {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .metric-label {
    font-size: 11px;
    color: #64748b;
  }
  .metric-value {
    font-size: 14px;
    font-weight: 600;
    color: #e2e8f0;
  }
  .error-badge {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
  }
  .error-badge.error {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
  }
  .anomaly-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .anomaly-item {
    padding: 12px 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .anomaly-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
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
    font-size: 13px;
    color: #94a3b8;
  }
  .anomaly-detail {
    font-size: 14px;
    color: #e2e8f0;
    margin-bottom: 4px;
  }
  .anomaly-trace {
    font-size: 12px;
    color: #64748b;
  }
  .trace-table {
    width: 100%;
    border-collapse: collapse;
  }
  .trace-table th, .trace-table td {
    padding: 12px;
    text-align: left;
    border-bottom: 1px solid #334155;
  }
  .trace-table th {
    font-size: 12px;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
  }
  .trace-id {
    font-family: monospace;
    color: #3b82f6;
  }
  .status-badge {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  .status-badge.ok { background: rgba(34, 197, 94, 0.1); color: #22c55e; }
  .status-badge.error { background: rgba(239, 68, 68, 0.1); color: #ef4444; }
  .empty {
    text-align: center;
    padding: 32px;
    color: #64748b;
  }
  @media (max-width: 1024px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
