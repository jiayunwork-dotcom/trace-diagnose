<script>
  import { onMount } from 'svelte'
  import Card from '../components/Card.svelte'
  import { batchCompare, getServices } from '../api'
  import { navigate } from 'svelte-routing'

  let services = []
  let results = []
  let loading = false
  let selectedService = ''

  let baselineStart = ''
  let baselineEnd = ''
  let comparisonStart = ''
  let comparisonEnd = ''

  onMount(async () => {
    try {
      const res = await getServices()
      services = res.data

      const now = new Date()
      const oneHourAgo = new Date(now.getTime() - 60 * 60 * 1000)
      const yesterday = new Date(now.getTime() - 24 * 60 * 60 * 1000)
      const yesterdayEnd = new Date(yesterday.getTime() + 60 * 60 * 1000)

      baselineStart = formatDateTimeLocal(yesterday)
      baselineEnd = formatDateTimeLocal(yesterdayEnd)
      comparisonStart = formatDateTimeLocal(oneHourAgo)
      comparisonEnd = formatDateTimeLocal(now)
    } catch (e) {
      console.error('Failed to load services:', e)
    }
  })

  function formatDateTimeLocal(date) {
    const pad = (n) => n.toString().padStart(2, '0')
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
  }

  async function handleCompare() {
    if (!baselineStart || !baselineEnd || !comparisonStart || !comparisonEnd) {
      alert('请填写完整的时间段')
      return
    }

    loading = true
    try {
      const res = await batchCompare({
        baseline_start: new Date(baselineStart).toISOString(),
        baseline_end: new Date(baselineEnd).toISOString(),
        comparison_start: new Date(comparisonStart).toISOString(),
        comparison_end: new Date(comparisonEnd).toISOString(),
      })
      results = res.data
    } catch (e) {
      console.error('Failed to compare:', e)
      alert('对比失败，请检查时间段是否正确')
    } finally {
      loading = false
    }
  }

  function getFilteredResults() {
    if (!selectedService) return results
    return results.filter(r => r.service_name === selectedService)
  }

  function formatDuration(ms) {
    if (ms < 1000) return `${ms.toFixed(0)}ms`
    return `${(ms / 1000).toFixed(2)}s`
  }

  function getChangeColor(pct) {
    if (pct >= 20) return 'color: #ef4444;'
    if (pct >= 10) return 'color: #f59e0b;'
    if (pct > 0) return 'color: #64748b;'
    return 'color: #10b981;'
  }
</script>

<div class="batch-compare-page">
  <div class="page-header">
    <div>
      <h1>批量Trace对比</h1>
      <p class="subtitle">对比两个时间段的服务性能，找出性能回归</p>
    </div>
  </div>

  <Card title="对比配置">
    <div class="config-form">
      <div class="time-section">
        <h3>基线时间段（如：上周同时段）</h3>
        <div class="time-inputs">
          <div class="form-group">
            <label>开始时间</label>
            <input type="datetime-local" bind:value={baselineStart} />
          </div>
          <div class="form-group">
            <label>结束时间</label>
            <input type="datetime-local" bind:value={baselineEnd} />
          </div>
        </div>
      </div>

      <div class="time-section">
        <h3>对比时间段（如：今天）</h3>
        <div class="time-inputs">
          <div class="form-group">
            <label>开始时间</label>
            <input type="datetime-local" bind:value={comparisonStart} />
          </div>
          <div class="form-group">
            <label>结束时间</label>
            <input type="datetime-local" bind:value={comparisonEnd} />
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn btn-primary" on:click={handleCompare} disabled={loading}>
          {loading ? '对比中...' : '开始对比'}
        </button>
      </div>
    </div>
  </Card>

  {#if results.length > 0}
    <Card title="对比结果">
      <div class="results-header">
        <div class="filter-item">
          <label>服务筛选</label>
          <select bind:value={selectedService}>
            <option value="">全部服务</option>
            {#each services as svc}
              <option value={svc.service_name}>{svc.service_name}</option>
            {/each}
          </select>
        </div>
        <div class="stats">
          <span class="stat-item regressions">
            性能回归: {getFilteredResults().filter(r => r.is_regression).length} 个
          </span>
          <span class="stat-item">
            总对比项: {getFilteredResults().length} 个
          </span>
        </div>
      </div>

      <div class="table-container">
        <table class="data-table">
          <thead>
            <tr>
              <th>服务</th>
              <th>操作</th>
              <th>基线平均延迟</th>
              <th>对比平均延迟</th>
              <th>变化率</th>
              <th>基线P95</th>
              <th>对比P95</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            {#each getFilteredResults() as result}
              <tr class="{result.is_regression ? 'regression-row' : ''}">
                <td>{result.service_name}</td>
                <td class="operation-name">{result.operation_name}</td>
                <td>{formatDuration(result.baseline_avg_duration)}</td>
                <td>{formatDuration(result.comparison_avg_duration)}</td>
                <td style={getChangeColor(result.duration_change_pct)}>
                  {result.duration_change_pct > 0 ? '+' : ''}{result.duration_change_pct.toFixed(1)}%
                </td>
                <td>{formatDuration(result.baseline_p95)}</td>
                <td>{formatDuration(result.comparison_p95)}</td>
                <td>
                  {#if result.is_regression}
                    <span class="badge badge-danger">性能回归</span>
                  {:else if result.duration_change_pct > 0}
                    <span class="badge badge-warning">轻微变慢</span>
                  {:else if result.duration_change_pct < 0}
                    <span class="badge badge-success">性能提升</span>
                  {:else}
                    <span class="badge badge-secondary">无变化</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </Card>
  {/if}

  {#if !loading && results.length === 0 && baselineStart}
    <Card>
      <div class="empty-state">
        <p>暂无对比数据，请调整时间段后重新对比</p>
      </div>
    </Card>
  {/if}
</div>

<style>
  .batch-compare-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .page-header h1 {
    margin: 0 0 8px 0;
    font-size: 24px;
    color: #1e293b;
  }

  .subtitle {
    margin: 0;
    color: #64748b;
    font-size: 14px;
  }

  .config-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .time-section {
    padding: 16px;
    background: #f8fafc;
    border-radius: 8px;
  }

  .time-section h3 {
    margin: 0 0 12px 0;
    font-size: 14px;
    color: #475569;
  }

  .time-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 13px;
    color: #64748b;
    font-weight: 500;
  }

  .form-group input,
  .form-group select {
    padding: 8px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 14px;
    background: white;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
  }

  .btn {
    padding: 10px 24px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding-bottom: 16px;
    border-bottom: 1px solid #e2e8f0;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .filter-item label {
    font-size: 13px;
    color: #64748b;
    font-weight: 500;
  }

  .filter-item select {
    padding: 6px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 14px;
    background: white;
  }

  .stats {
    display: flex;
    gap: 16px;
  }

  .stat-item {
    font-size: 13px;
    color: #64748b;
  }

  .stat-item.regressions {
    color: #ef4444;
    font-weight: 500;
  }

  .table-container {
    overflow-x: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
  }

  .data-table th {
    background: #f8fafc;
    padding: 12px 16px;
    text-align: left;
    font-weight: 600;
    font-size: 13px;
    color: #64748b;
    border-bottom: 2px solid #e2e8f0;
  }

  .data-table td {
    padding: 12px 16px;
    border-bottom: 1px solid #e2e8f0;
    font-size: 14px;
  }

  .operation-name {
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .regression-row {
    background: #fef2f2;
  }

  .badge {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
  }

  .badge-danger {
    background: #fef2f2;
    color: #dc2626;
  }

  .badge-warning {
    background: #fffbeb;
    color: #d97706;
  }

  .badge-success {
    background: #f0fdf4;
    color: #16a34a;
  }

  .badge-secondary {
    background: #f1f5f9;
    color: #64748b;
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: #94a3b8;
  }
</style>
