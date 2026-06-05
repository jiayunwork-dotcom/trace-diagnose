<script>
  import { onMount } from 'svelte'
  import Card from '../components/Card.svelte'
  import { getSLOs, createSLO, deleteSLO, getSLOStatus, getServices } from '../api.js'

  let slos = []
  let services = []
  let loading = true
  let showForm = false
  let newSLO = {
    service_name: '',
    slo_type: 'latency_p95',
    threshold: 500,
    target: 99.9,
    window_days: 30,
    description: '',
  }

  onMount(async () => {
    await loadData()
  })

  async function loadData() {
    loading = true
    try {
      const [sloRes, svcRes] = await Promise.all([
        getSLOs(),
        getServices(),
      ])
      slos = sloRes.data
      services = svcRes.data
    } catch (e) {
      console.error('Failed to load SLOs:', e)
    } finally {
      loading = false
    }
  }

  async function handleCreate() {
    try {
      await createSLO(newSLO)
      showForm = false
      newSLO = {
        service_name: '',
        slo_type: 'latency_p95',
        threshold: 500,
        target: 99.9,
        window_days: 30,
        description: '',
      }
      await loadData()
    } catch (e) {
      alert('创建SLO失败')
    }
  }

  async function handleDelete(id) {
    if (confirm('确定删除此SLO吗？')) {
      try {
        await deleteSLO(id)
        await loadData()
      } catch (e) {
        console.error('Failed to delete SLO:', e)
      }
    }
  }

  function getBudgetColor(remaining) {
    if (remaining > 50) return 'text-green-400'
    if (remaining > 20) return 'text-yellow-400'
    return 'text-red-400'
  }
</script>

<div class="slo-page">
  <div class="page-header">
    <div>
      <h1>SLO监控</h1>
      <p class="subtitle">管理服务级别目标和错误预算</p>
    </div>
    <button class="add-btn" on:click={() => showForm = !showForm}>
      {showForm ? '取消' : '+ 新建SLO'}
    </button>
  </div>

  {#if showForm}
    <Card title="新建SLO">
      <div class="slo-form">
        <div class="form-row">
          <div class="form-item">
            <label>服务</label>
            <select bind:value={newSLO.service_name}>
              <option value="">选择服务</option>
              {#each services as svc}
                <option value={svc.service_name}>{svc.service_name}</option>
              {/each}
            </select>
          </div>
          <div class="form-item">
            <label>SLO类型</label>
            <select bind:value={newSLO.slo_type}>
              <option value="latency_p95">P95延迟</option>
              <option value="error_rate">错误率</option>
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-item">
            <label>阈值</label>
            <input type="number" bind:value={newSLO.threshold} />
          </div>
          <div class="form-item">
            <label>目标 (%)</label>
            <input type="number" step="0.1" bind:value={newSLO.target} />
          </div>
          <div class="form-item">
            <label>窗口 (天)</label>
            <input type="number" bind:value={newSLO.window_days} />
          </div>
        </div>
        <div class="form-item">
          <label>描述</label>
          <input type="text" bind:value={newSLO.description} placeholder="可选" />
        </div>
        <button class="submit-btn" on:click={handleCreate}>创建</button>
      </div>
    </Card>
  {/if}

  {#if loading}
    <div class="loading">加载中...</div>
  {:else if slos.length === 0}
    <Card>
      <div class="empty">暂无SLO配置，点击上方按钮创建</div>
    </Card>
  {:else}
    <div class="slo-list">
      {#each slos as slo}
        <Card title={`${slo.service_name} - ${slo.slo_type === 'latency_p95' ? 'P95延迟' : '错误率'}`}>
          <div class="slo-card">
            <div class="slo-metrics">
              <div class="metric">
                <span class="metric-label">阈值</span>
                <span class="metric-value">
                  {slo.threshold}{slo.slo_type === 'latency_p95' ? 'ms' : '%'}
                </span>
              </div>
              <div class="metric">
                <span class="metric-label">目标</span>
                <span class="metric-value">{slo.target}%</span>
              </div>
              <div class="metric">
                <span class="metric-label">窗口</span>
                <span class="metric-value">{slo.window_days}天</span>
              </div>
              <div class="metric">
                <span class="metric-label">错误预算</span>
                <span class="metric-value budget">
                  {(100 - slo.target).toFixed(1)}%
                </span>
              </div>
            </div>
            {#if slo.description}
              <div class="slo-desc">{slo.description}</div>
            {/if}
            <div class="slo-actions">
              <button class="delete-btn" on:click={() => handleDelete(slo.id)}>删除</button>
            </div>
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>

<style>
  .slo-page {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
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
  .add-btn {
    padding: 10px 20px;
    background: #3b82f6;
    border: none;
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .add-btn:hover {
    background: #2563eb;
  }
  .slo-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .form-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
  }
  .form-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-item label {
    font-size: 13px;
    color: #94a3b8;
    font-weight: 500;
  }
  .form-item select, .form-item input {
    padding: 10px 12px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
  }
  .submit-btn {
    align-self: flex-start;
    padding: 10px 24px;
    background: #22c55e;
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .submit-btn:hover {
    background: #16a34a;
  }
  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .slo-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: 16px;
  }
  .slo-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .slo-metrics {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }
  .metric {
    text-align: center;
    padding: 12px;
    background: #0f172a;
    border-radius: 8px;
  }
  .metric-label {
    display: block;
    font-size: 12px;
    color: #94a3b8;
    margin-bottom: 4px;
  }
  .metric-value {
    font-size: 18px;
    font-weight: 600;
    color: #e2e8f0;
  }
  .metric-value.budget {
    color: #f59e0b;
  }
  .slo-desc {
    font-size: 13px;
    color: #94a3b8;
    padding: 8px 12px;
    background: #0f172a;
    border-radius: 6px;
  }
  .slo-actions {
    display: flex;
    justify-content: flex-end;
  }
  .delete-btn {
    padding: 6px 14px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #ef4444;
    font-size: 13px;
    cursor: pointer;
  }
  .delete-btn:hover {
    background: rgba(239, 68, 68, 0.2);
  }
</style>
