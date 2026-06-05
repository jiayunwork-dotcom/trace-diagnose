<script>
  import { onMount } from 'svelte'
  import { Link } from 'svelte-routing'
  import Card from '../components/Card.svelte'
  import { getTrace, getCriticalPath } from '../api.js'

  export let params = {}
  let traceId = params.id

  let traceData = null
  let criticalPath = null
  let loading = true
  let activeTab = 'waterfall'

  onMount(async () => {
    try {
      const [traceRes, pathRes] = await Promise.all([
        getTrace(traceId),
        getCriticalPath(traceId).catch(() => null),
      ])
      traceData = traceRes.data
      criticalPath = pathRes?.data
    } catch (e) {
      console.error('Failed to load trace:', e)
    } finally {
      loading = false
    }
  })

  function formatDuration(ms) {
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(2)}s`
  }

  $: if (traceData) {
    const spans = traceData.spans || []
    const minTime = spans.length > 0 ? Math.min(...spans.map(s => new Date(s.start_time).getTime())) : 0
    const maxTime = spans.length > 0 ? Math.max(...spans.map(s => new Date(s.end_time).getTime())) : 0
    const totalDuration = maxTime - minTime
  }
</script>

<div class="trace-detail">
  <div class="page-header">
    <Link to="/traces" class="back-link">← 返回列表</Link>
    <h1>Trace详情</h1>
    <p class="trace-id-display">ID: {traceId}</p>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else if !traceData}
    <div class="empty">Trace不存在</div>
  {:else}
    <div class="trace-summary">
      <div class="summary-item">
        <span class="label">总持续时间</span>
        <span class="value">{formatDuration(traceData.trace.duration_ms || 0)}</span>
      </div>
      <div class="summary-item">
        <span class="label">Span数量</span>
        <span class="value">{traceData.trace.span_count}</span>
      </div>
      <div class="summary-item">
        <span class="label">服务数量</span>
        <span class="value">{traceData.trace.service_count}</span>
      </div>
      <div class="summary-item">
        <span class="label">开始时间</span>
        <span class="value">{new Date(traceData.trace.start_time).toLocaleString()}</span>
      </div>
      <div class="summary-item">
        <span class="label">状态</span>
        <span class={`value ${traceData.trace.has_errors ? 'error' : 'ok'}`}>
          {traceData.trace.has_errors ? '有错误' : '正常'}
        </span>
      </div>
    </div>

    <div class="tabs">
      <button class="tab-btn" class:active={activeTab === 'waterfall'} on:click={() => activeTab = 'waterfall'}>
        瀑布图
      </button>
      <button class="tab-btn" class:active={activeTab === 'critical'} on:click={() => activeTab = 'critical'}>
        关键路径
      </button>
      <button class="tab-btn" class:active={activeTab === 'spans'} on:click={() => activeTab = 'spans'}>
        Span列表
      </button>
    </div>

    {#if activeTab === 'waterfall'}
      <Card title="调用瀑布图">
        <div class="waterfall-container">
          {#if traceData.spans.length === 0}
            <div class="empty">无Span数据</div>
          {:else}
            <div class="waterfall-chart">
              {#each traceData.spans as span, idx}
                <div class="waterfall-row">
                  <div class="row-label">
                    <span class="service-tag">{span.service_name}</span>
                    <span class="operation-name">{span.operation_name}</span>
                  </div>
                  <div class="row-bar-container">
                    <div
                      class="row-bar"
                      class:error={span.status_code >= 400}
                      style="left: {((new Date(span.start_time).getTime() - new Date(traceData.trace.start_time).getTime()) / (traceData.trace.duration_ms || 1) * 100)}%; width: {(span.duration_ms / (traceData.trace.duration_ms || 1) * 100)}%;"
                      title={`${span.duration_ms}ms`}
                    >
                      <span class="bar-duration">{span.duration_ms}ms</span>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </Card>
    {/if}

    {#if activeTab === 'critical'}
      <Card title="关键路径分析">
        {#if !criticalPath || criticalPath.spans.length === 0}
          <div class="empty">暂无可分析的关键路径</div>
        {:else}
          <div class="critical-path-info">
            <p>总延迟: <strong>{formatDuration(criticalPath.total_duration)}</strong></p>
          </div>
          <div class="critical-path-list">
            {#each criticalPath.spans as span}
              <div class="critical-path-item">
                <div class="path-item-header">
                  <span class="service-tag">{span.service_name}</span>
                  <span class="contribution">{span.contribution_pct.toFixed(1)}%</span>
                </div>
                <div class="path-item-detail">
                  {span.operation_name} · {formatDuration(span.duration_ms)}
                </div>
                <div class="contribution-bar">
                  <div class="contribution-fill" style="width: {span.contribution_pct}%"></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </Card>
    {/if}

    {#if activeTab === 'spans'}
      <Card title="Span详情列表">
        <div class="span-list">
          {#each traceData.spans as span}
            <div class="span-item">
              <div class="span-header">
                <span class="service-tag">{span.service_name}</span>
                <span class="span-name">{span.operation_name}</span>
                <span class="span-duration">{formatDuration(span.duration_ms)}</span>
                <span class={`status-tag ${span.status_code >= 400 ? 'error' : 'ok'}`}>
                  {span.status_code}
                </span>
              </div>
              <div class="span-detail">
                <span>Span ID: {span.span_id.slice(0, 16)}...</span>
                {#if span.parent_span_id}
                  <span>Parent: {span.parent_span_id.slice(0, 16)}...</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </Card>
    {/if}
  {/if}
</div>

<style>
  .trace-detail {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .back-link {
    color: #3b82f6;
    text-decoration: none;
    font-size: 14px;
    margin-bottom: 8px;
    display: inline-block;
  }
  .back-link:hover {
    text-decoration: underline;
  }
  .page-header h1 {
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
    margin: 0 0 4px 0;
  }
  .trace-id-display {
    color: #64748b;
    font-family: monospace;
    font-size: 14px;
    margin: 0;
  }
  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .trace-summary {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }
  .summary-item {
    flex: 1;
    min-width: 140px;
    padding: 16px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 13px;
    color: #94a3b8;
  }
  .value {
    font-size: 18px;
    font-weight: 600;
    color: #f1f5f9;
  }
  .value.ok { color: #22c55e; }
  .value.error { color: #ef4444; }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid #334155;
  }
  .tab-btn {
    padding: 12px 20px;
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }
  .tab-btn:hover {
    color: #e2e8f0;
  }
  .tab-btn.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }
  .waterfall-container {
    overflow-x: auto;
  }
  .waterfall-chart {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 600px;
  }
  .waterfall-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .row-label {
    width: 280px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .service-tag {
    padding: 2px 6px;
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
  }
  .operation-name {
    color: #e2e8f0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-bar-container {
    flex: 1;
    height: 24px;
    background: #0f172a;
    border-radius: 4px;
    position: relative;
    min-width: 300px;
  }
  .row-bar {
    position: absolute;
    height: 100%;
    background: #3b82f6;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding-right: 8px;
    min-width: 40px;
  }
  .row-bar.error {
    background: #ef4444;
  }
  .bar-duration {
    font-size: 11px;
    color: white;
    font-weight: 500;
  }
  .critical-path-info {
    margin-bottom: 16px;
    font-size: 14px;
    color: #e2e8f0;
  }
  .critical-path-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .critical-path-item {
    padding: 12px 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .path-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .contribution {
    font-weight: 600;
    color: #3b82f6;
  }
  .path-item-detail {
    font-size: 13px;
    color: #94a3b8;
    margin-bottom: 8px;
  }
  .contribution-bar {
    height: 6px;
    background: #1e293b;
    border-radius: 3px;
    overflow: hidden;
  }
  .contribution-fill {
    height: 100%;
    background: #3b82f6;
    border-radius: 3px;
  }
  .span-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .span-item {
    padding: 12px 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .span-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 6px;
  }
  .span-name {
    flex: 1;
    color: #e2e8f0;
    font-weight: 500;
  }
  .span-duration {
    color: #94a3b8;
    font-size: 13px;
  }
  .status-tag {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
  }
  .status-tag.ok { background: rgba(34, 197, 94, 0.2); color: #22c55e; }
  .status-tag.error { background: rgba(239, 68, 68, 0.2); color: #ef4444; }
  .span-detail {
    font-size: 12px;
    color: #64748b;
    display: flex;
    gap: 16px;
  }
</style>
