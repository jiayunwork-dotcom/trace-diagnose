<script>
  import { onMount } from 'svelte'
  import Card from '../components/Card.svelte'
  import { uploadTraceFile, getImportJobs, getImportProgress } from '../api.js'

  let formats = [
    { value: 'otel', label: 'OpenTelemetry JSON' },
    { value: 'jaeger', label: 'Jaeger JSON' },
    { value: 'zipkin', label: 'Zipkin v2 JSON' },
  ]
  let selectedFormat = 'otel'
  let dragging = false
  let jobs = []
  let loading = true
  let currentJobId = null
  let currentProgress = null

  onMount(async () => {
    await loadJobs()
  })

  async function loadJobs() {
    try {
      const res = await getImportJobs()
      jobs = res.data
    } catch (e) {
      console.error('Failed to load jobs:', e)
    } finally {
      loading = false
    }
  }

  function handleDragOver(e) {
    e.preventDefault()
    dragging = true
  }

  function handleDragLeave() {
    dragging = false
  }

  function handleDrop(e) {
    e.preventDefault()
    dragging = false
    const files = e.dataTransfer.files
    if (files.length > 0) {
      uploadFile(files[0])
    }
  }

  function handleFileSelect(e) {
    const file = e.target.files[0]
    if (file) {
      uploadFile(file)
    }
  }

  async function uploadFile(file) {
    try {
      const res = await uploadTraceFile(file, selectedFormat)
      currentJobId = res.data.job_id
      pollProgress()
    } catch (e) {
      alert('上传失败: ' + e.message)
    }
  }

  async function pollProgress() {
    if (!currentJobId) return
    try {
      const res = await getImportProgress(currentJobId)
      currentProgress = res.data
      if (res.data.status === 'processing') {
        setTimeout(pollProgress, 1000)
      } else {
        setTimeout(loadJobs, 500)
      }
    } catch (e) {
      console.error('Failed to get progress:', e)
    }
  }

  function getStatusClass(status) {
    switch (status) {
      case 'completed': return 'status-completed'
      case 'processing': return 'status-processing'
      case 'failed': return 'status-failed'
      default: return 'status-pending'
    }
  }

  function getStatusText(status) {
    switch (status) {
      case 'completed': return '已完成'
      case 'processing': return '处理中'
      case 'failed': return '失败'
      default: return '等待中'
    }
  }
</script>

<div class="import-page">
  <div class="page-header">
    <h1>数据导入</h1>
    <p class="subtitle">导入分布式追踪数据进行分析</p>
  </div>

  <div class="import-layout">
    <Card title="上传文件">
      <div class="upload-section">
        <div class="format-select">
          <label>数据格式</label>
          <select bind:value={selectedFormat}>
            {#each formats as fmt}
              <option value={fmt.value}>{fmt.label}</option>
            {/each}
          </select>
        </div>

        <div 
          class="drop-zone" 
          class:dragging={dragging}
          on:dragover={handleDragOver}
          on:dragleave={handleDragLeave}
          on:drop={handleDrop}
        >
          <div class="drop-icon">📁</div>
          <p class="drop-text">拖拽文件到此处，或</p>
          <label class="browse-btn">
            选择文件
            <input type="file" accept=".json,.bin" on:change={handleFileSelect} hidden />
          </label>
          <p class="drop-hint">支持 JSON 和 Protobuf 格式，最大 100MB</p>
        </div>

        {#if currentProgress}
          <div class="progress-section">
            <div class="progress-header">
              <span>导入进度</span>
              <span>{currentProgress.processed} / {currentProgress.total || '?'}</span>
            </div>
            <div class="progress-bar">
              <div 
                class="progress-fill" 
                style="width: {currentProgress.total ? (currentProgress.processed / currentProgress.total * 100) : 0}%;"
              />
            </div>
            <div class="progress-status">{getStatusText(currentProgress.status)}</div>
          </div>
        {/if}
      </div>
    </Card>

    <Card title="导入历史">
      {#if loading}
        <div class="loading">加载中...</div>
      {:else if jobs.length === 0}
        <div class="empty">暂无导入记录</div>
      {:else}
        <div class="job-list">
          {#each jobs as job}
            <div class="job-item">
              <div class="job-main">
                <span class="job-name">{job.file_name || 'API推送'}</span>
                <span class={`job-status ${getStatusClass(job.status)}`}>
                  {getStatusText(job.status)}
                </span>
              </div>
              <div class="job-detail">
                <span>{job.format}</span>
                <span>{job.processed_spans} / {job.total_spans} spans</span>
                <span>{new Date(job.created_at).toLocaleString()}</span>
              </div>
              {#if job.error_message}
                <div class="job-error">{job.error_message}</div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </Card>
  </div>
</div>

<style>
  .import-page {
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
  .import-layout {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .upload-section {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .format-select {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .format-select label {
    font-size: 13px;
    color: #94a3b8;
    font-weight: 500;
  }
  .format-select select {
    padding: 10px 12px;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 14px;
  }
  .drop-zone {
    border: 2px dashed #334155;
    border-radius: 12px;
    padding: 40px 20px;
    text-align: center;
    transition: all 0.2s;
    background: #0f172a;
  }
  .drop-zone.dragging {
    border-color: #3b82f6;
    background: rgba(59, 130, 246, 0.05);
  }
  .drop-icon {
    font-size: 48px;
    margin-bottom: 12px;
  }
  .drop-text {
    color: #e2e8f0;
    font-size: 14px;
    margin: 0 0 8px 0;
  }
  .browse-btn {
    display: inline-block;
    padding: 8px 20px;
    background: #3b82f6;
    color: white;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    margin-bottom: 8px;
  }
  .browse-btn:hover {
    background: #2563eb;
  }
  .drop-hint {
    color: #64748b;
    font-size: 12px;
    margin: 0;
  }
  .progress-section {
    padding: 16px;
    background: #0f172a;
    border-radius: 8px;
  }
  .progress-header {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    color: #e2e8f0;
    margin-bottom: 8px;
  }
  .progress-bar {
    height: 8px;
    background: #1e293b;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 8px;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b82f6, #22c55e);
    border-radius: 4px;
    transition: width 0.3s;
  }
  .progress-status {
    font-size: 12px;
    color: #22c55e;
    text-align: center;
  }
  .loading, .empty {
    text-align: center;
    padding: 48px;
    color: #94a3b8;
  }
  .job-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 500px;
    overflow-y: auto;
  }
  .job-item {
    padding: 12px;
    background: #0f172a;
    border-radius: 8px;
  }
  .job-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .job-name {
    font-weight: 500;
    color: #e2e8f0;
    font-size: 14px;
  }
  .job-status {
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  .status-pending { background: rgba(148, 163, 184, 0.1); color: #94a3b8; }
  .status-processing { background: rgba(59, 130, 246, 0.1); color: #3b82f6; }
  .status-completed { background: rgba(34, 197, 94, 0.1); color: #22c55e; }
  .status-failed { background: rgba(239, 68, 68, 0.1); color: #ef4444; }
  .job-detail {
    display: flex;
    gap: 16px;
    font-size: 12px;
    color: #64748b;
  }
  .job-error {
    margin-top: 6px;
    padding: 6px 10px;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 4px;
    font-size: 12px;
    color: #ef4444;
  }
  @media (max-width: 1024px) {
    .import-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
