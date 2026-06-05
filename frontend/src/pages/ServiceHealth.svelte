<script>
  import { onMount } from 'svelte'
  import { Line, Radar } from 'svelte-chartjs'
  import {
    Chart as ChartJS,
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    RadialLinearScale,
    Filler,
    Title,
    Tooltip,
    Legend,
  } from 'chart.js'
  import Card from '../components/Card.svelte'
  import {
    getHealthRankings,
    getServiceHealthTrend,
    getCapacityPlans,
    getHealthEvents,
    computeHealthNow,
  } from '../api.js'

  ChartJS.register(
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    RadialLinearScale,
    Filler,
    Title,
    Tooltip,
    Legend
  )

  let rankings = []
  let capacityPlans = []
  let events = []
  let selectedService = null
  let trendData = []
  let loading = true
  let activeTab = 'rankings'

  onMount(async () => {
    await loadAllData()
  })

  async function loadAllData() {
    loading = true
    try {
      const [rankRes, capRes, eventRes] = await Promise.all([
        getHealthRankings(),
        getCapacityPlans(),
        getHealthEvents(),
      ])
      rankings = rankRes.data
      capacityPlans = capRes.data
      events = eventRes.data
    } catch (e) {
      console.error('Failed to load health data:', e)
    } finally {
      loading = false
    }
  }

  async function loadServiceTrend(serviceName) {
    try {
      const res = await getServiceHealthTrend(serviceName, { days: 7 })
      trendData = res.data
    } catch (e) {
      console.error('Failed to load trend data:', e)
      trendData = []
    }
  }

  async function handleComputeNow() {
    if (confirm('确定要立即重新计算所有服务的健康评分吗？')) {
      try {
        await computeHealthNow({})
        alert('健康评分计算已在后台启动，请稍后刷新查看结果')
      } catch (e) {
        alert('启动计算失败')
      }
    }
  }

  async function selectService(service) {
    if (selectedService?.service_name === service.service_name) {
      selectedService = null
      trendData = []
    } else {
      selectedService = service
      await loadServiceTrend(service.service_name)
    }
  }

  function getScoreColor(score) {
    if (score >= 80) return '#22c55e'
    if (score >= 60) return '#f59e0b'
    return '#ef4444'
  }

  function getScoreBgColor(score) {
    if (score >= 80) return 'rgba(34, 197, 94, 0.1)'
    if (score >= 60) return 'rgba(245, 158, 11, 0.1)'
    return 'rgba(239, 68, 68, 0.1)'
  }

  function getStatusBadge(status) {
    const styles = {
      healthy: { bg: 'rgba(34, 197, 94, 0.15)', color: '#22c55e', text: '健康' },
      warning: { bg: 'rgba(245, 158, 11, 0.15)', color: '#f59e0b', text: '警告' },
      danger: { bg: 'rgba(239, 68, 68, 0.15)', color: '#ef4444', text: '危险' },
    }
    return styles[status] || styles.healthy
  }

  function clampScore(score) {
    return Math.max(0, Math.min(100, score || 0))
  }

  $: radarData = selectedService ? {
    labels: ['可用性', '延迟', '吞吐稳定性', '错误多样性'],
    datasets: [
      {
        label: '得分',
        data: [
          Math.max(0, Math.min(100, selectedService.availability_score || 0)),
          Math.max(0, Math.min(100, selectedService.latency_score || 0)),
          Math.max(0, Math.min(100, selectedService.throughput_stability_score || 0)),
          Math.max(0, Math.min(100, selectedService.error_diversity_score || 0)),
        ],
        backgroundColor: 'rgba(59, 130, 246, 0.2)',
        borderColor: '#3b82f6',
        borderWidth: 2,
        pointBackgroundColor: '#3b82f6',
        pointBorderColor: '#fff',
        pointRadius: 4,
        fill: true,
      },
    ],
  } : null

  $: radarOptions = {
    responsive: true,
    maintainAspectRatio: true,
    scales: {
      r: {
        type: 'radialLinear',
        beginAtZero: true,
        min: 0,
        max: 100,
        suggestedMin: 0,
        suggestedMax: 100,
        ticks: {
          stepSize: 20,
          color: '#94a3b8',
          backdropColor: 'transparent',
          showLabelBackdrop: false,
        },
        grid: {
          color: 'rgba(148, 163, 184, 0.2)',
          circular: true,
        },
        angleLines: {
          color: 'rgba(148, 163, 184, 0.3)',
          lineWidth: 1,
        },
        pointLabels: {
          color: '#e2e8f0',
          font: { size: 13, weight: '500' },
          padding: 15,
        },
      },
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        callbacks: {
          label: function(context) {
            return `${context.label}: ${context.raw.toFixed(1)}分`
          }
        }
      }
    },
    elements: {
      line: {
        tension: 0.1,
      }
    }
  }

  $: trendChartData = trendData.length > 0 ? {
    labels: trendData.map(d => new Date(d.snapshot_time).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: 'numeric' })),
    datasets: [
      {
        label: '总分',
        data: trendData.map(d => d.total_score),
        borderColor: '#3b82f6',
        backgroundColor: 'rgba(59, 130, 246, 0.1)',
        fill: true,
        tension: 0.4,
      },
      {
        label: '可用性',
        data: trendData.map(d => d.availability_score),
        borderColor: '#22c55e',
        backgroundColor: 'transparent',
        tension: 0.4,
        borderDash: [5, 5],
      },
      {
        label: '延迟',
        data: trendData.map(d => d.latency_score),
        borderColor: '#f59e0b',
        backgroundColor: 'transparent',
        tension: 0.4,
        borderDash: [5, 5],
      },
    ],
  } : null

  $: trendOptions = {
    responsive: true,
    scales: {
      y: {
        beginAtZero: true,
        max: 100,
        grid: { color: 'rgba(148, 163, 184, 0.1)' },
        ticks: { color: '#94a3b8' },
      },
      x: {
        grid: { display: false },
        ticks: { color: '#94a3b8', maxRotation: 45 },
      },
    },
    plugins: {
      legend: {
        labels: { color: '#e2e8f0' },
      },
    },
    interaction: {
      intersect: false,
      mode: 'index',
    },
  }
</script>

<div class="health-page">
  <div class="page-header">
    <div>
      <h1>服务健康</h1>
      <p class="subtitle">健康评分与容量规划</p>
    </div>
    <button class="compute-btn" on:click={handleComputeNow}>
      🔄 立即计算
    </button>
  </div>

  <div class="tabs">
    <button class="tab-btn" class:active={activeTab === 'rankings'} on:click={() => activeTab = 'rankings'}>
      🏆 健康排行榜
    </button>
    <button class="tab-btn" class:active={activeTab === 'capacity'} on:click={() => activeTab = 'capacity'}>
      📊 容量规划
    </button>
    <button class="tab-btn" class:active={activeTab === 'events'} on:click={() => activeTab = 'events'}>
      🔔 健康事件
    </button>
  </div>

  {#if loading}
    <div class="loading">加载中...</div>
  {:else}
    {#if activeTab === 'rankings'}
      <div class="rankings-section">
        {#if rankings.length === 0}
          <Card>
            <div class="empty">暂无健康评分数据，点击"立即计算"生成</div>
          </Card>
        {:else}
          <div class="rankings-grid">
            {#each rankings as item}
              <div
                class="service-card"
                class:expanded={selectedService?.service_name === item.service_name}
                style="border-left: 4px solid {getScoreColor(clampScore(item.total_score))}"
                on:click={() => selectService(item)}
              >
                <div class="card-header">
                  <div class="service-info">
                    <span class="service-name">{item.service_name}</span>
                    <span class="status-badge" style="background: {getStatusBadge(item.status).bg}; color: {getStatusBadge(item.status).color}">
                      {getStatusBadge(item.status).text}
                    </span>
                  </div>
                  <div class="score-circle" style="background: {getScoreBgColor(clampScore(item.total_score))}">
                    <span class="score-value" style="color: {getScoreColor(clampScore(item.total_score))}">
                      {clampScore(item.total_score).toFixed(0)}
                    </span>
                  </div>
                </div>

                <div class="score-bars">
                  <div class="score-bar-item">
                    <span class="bar-label">可用性</span>
                    <div class="bar-track">
                      <div class="bar-fill" style="width: {clampScore(item.availability_score)}%; background: #22c55e"></div>
                    </div>
                    <span class="bar-value">{clampScore(item.availability_score).toFixed(0)}</span>
                  </div>
                  <div class="score-bar-item">
                    <span class="bar-label">延迟</span>
                    <div class="bar-track">
                      <div class="bar-fill" style="width: {clampScore(item.latency_score)}%; background: #f59e0b"></div>
                    </div>
                    <span class="bar-value">{clampScore(item.latency_score).toFixed(0)}</span>
                  </div>
                  <div class="score-bar-item">
                    <span class="bar-label">吞吐稳定</span>
                    <div class="bar-track">
                      <div class="bar-fill" style="width: {clampScore(item.throughput_stability_score)}%; background: #8b5cf6"></div>
                    </div>
                    <span class="bar-value">{clampScore(item.throughput_stability_score).toFixed(0)}</span>
                  </div>
                  <div class="score-bar-item">
                    <span class="bar-label">错误多样</span>
                    <div class="bar-track">
                      <div class="bar-fill" style="width: {clampScore(item.error_diversity_score)}%; background: #ec4899"></div>
                    </div>
                    <span class="bar-value">{clampScore(item.error_diversity_score).toFixed(0)}</span>
                  </div>
                </div>

                {#if selectedService?.service_name === item.service_name}
                  <div class="detail-panel">
                    <div class="detail-section">
                      <h4>四维雷达图</h4>
                      {#if radarData}
                        <div class="chart-container">
                          <Radar data={radarData} options={radarOptions} />
                        </div>
                      {/if}
                    </div>
                    <div class="detail-section">
                      <h4>7天健康趋势</h4>
                      {#if trendChartData}
                        <div class="chart-container">
                          <Line data={trendChartData} options={trendOptions} />
                        </div>
                      {:else}
                        <div class="empty-small">暂无趋势数据</div>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {:else if activeTab === 'capacity'}
      <Card title="容量规划">
        {#if capacityPlans.length === 0}
          <div class="empty">暂无容量规划数据</div>
        {:else}
          <div class="table-container">
            <table class="data-table">
              <thead>
                <tr>
                  <th>服务名称</th>
                  <th>当前QPS</th>
                  <th>预估最大QPS</th>
                  <th>剩余容量</th>
                  <th>平均响应时间</th>
                  <th>并发峰值(P95)</th>
                  <th>状态</th>
                </tr>
              </thead>
              <tbody>
                {#each capacityPlans as plan}
                  <tr class:warning={plan.is_warning}>
                    <td>{plan.service_name}</td>
                    <td>{plan.current_qps.toFixed(2)}</td>
                    <td>{plan.max_qps.toFixed(2)}</td>
                    <td>
                      <span class={plan.remaining_capacity < plan.current_qps * 0.2 ? 'text-danger' : 'text-success'}>
                        {plan.remaining_capacity.toFixed(2)}
                      </span>
                    </td>
                    <td>{plan.avg_response_time_ms.toFixed(0)}ms</td>
                    <td>{plan.concurrent_peak_p95}</td>
                    <td>
                      {#if plan.is_warning}
                        <span class="badge badge-danger">容量预警</span>
                      {:else}
                        <span class="badge badge-success">正常</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </Card>
    {:else if activeTab === 'events'}
      <Card title="健康事件">
        {#if events.length === 0}
          <div class="empty">暂无健康事件</div>
        {:else}
          <div class="events-list">
            {#each events as event}
              <div class="event-item">
                <div class="event-icon">
                  {event.severity === 'critical' ? '🔴' : event.severity === 'warning' ? '🟡' : '🟢'}
                </div>
                <div class="event-content">
                  <div class="event-header">
                    <span class="event-service">{event.service_name}</span>
                    <span class="event-time">{new Date(event.created_at).toLocaleString('zh-CN')}</span>
                  </div>
                  <div class="event-message">{event.message}</div>
                  {#if event.score !== null}
                    <div class="event-meta">
                      当前得分: {event.score?.toFixed(1)} / 阈值: {event.threshold}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </Card>
    {/if}
  {/if}
</div>

<style>
  .health-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
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

  .compute-btn {
    padding: 10px 20px;
    background: #3b82f6;
    border: none;
    border-radius: 8px;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .compute-btn:hover {
    background: #2563eb;
  }

  .tabs {
    display: flex;
    gap: 8px;
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
    margin-bottom: -1px;
    transition: all 0.2s;
  }

  .tab-btn:hover {
    color: #e2e8f0;
  }

  .tab-btn.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }

  .empty-small {
    text-align: center;
    padding: 24px;
    color: #94a3b8;
    font-size: 13px;
  }

  .rankings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
    gap: 16px;
  }

  .service-card {
    background: #1e293b;
    border-radius: 12px;
    padding: 20px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .service-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .service-card.expanded {
    grid-column: span 1;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }

  .service-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .service-name {
    font-size: 16px;
    font-weight: 600;
    color: #f1f5f9;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 10px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    width: fit-content;
  }

  .score-circle {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .score-value {
    font-size: 22px;
    font-weight: 700;
  }

  .score-bars {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .score-bar-item {
    display: grid;
    grid-template-columns: 70px 1fr 36px;
    align-items: center;
    gap: 10px;
  }

  .bar-label {
    font-size: 12px;
    color: #94a3b8;
  }

  .bar-track {
    height: 6px;
    background: #334155;
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s;
  }

  .bar-value {
    font-size: 12px;
    color: #e2e8f0;
    text-align: right;
    font-weight: 500;
  }

  .detail-panel {
    margin-top: 20px;
    padding-top: 20px;
    border-top: 1px solid #334155;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }

  .detail-section h4 {
    font-size: 14px;
    font-weight: 600;
    color: #e2e8f0;
    margin: 0 0 12px 0;
  }

  .chart-container {
    height: 250px;
  }

  .table-container {
    overflow-x: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
  }

  .data-table th {
    text-align: left;
    padding: 12px 16px;
    background: #0f172a;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 500;
    border-bottom: 1px solid #334155;
  }

  .data-table td {
    padding: 12px 16px;
    border-bottom: 1px solid #334155;
    color: #e2e8f0;
    font-size: 14px;
  }

  .data-table tr.warning {
    background: rgba(239, 68, 68, 0.05);
  }

  .text-success {
    color: #22c55e;
    font-weight: 500;
  }

  .text-danger {
    color: #ef4444;
    font-weight: 500;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 10px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
  }

  .badge-success {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
  }

  .badge-danger {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }

  .events-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .event-item {
    display: flex;
    gap: 14px;
    padding: 14px;
    background: #0f172a;
    border-radius: 8px;
    border-left: 3px solid #3b82f6;
  }

  .event-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .event-content {
    flex: 1;
    min-width: 0;
  }

  .event-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .event-service {
    font-weight: 600;
    color: #f1f5f9;
    font-size: 14px;
  }

  .event-time {
    font-size: 12px;
    color: #64748b;
  }

  .event-message {
    color: #e2e8f0;
    font-size: 13px;
    line-height: 1.5;
  }

  .event-meta {
    margin-top: 6px;
    font-size: 12px;
    color: #94a3b8;
  }
</style>
