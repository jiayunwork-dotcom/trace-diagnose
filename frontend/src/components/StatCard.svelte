<script>
  export let label = ''
  export let value = ''
  export let trend = null
  export let icon = ''
  export let color = 'blue'

  const colorClasses = {
    blue: 'bg-blue-500/10 text-blue-400',
    green: 'bg-green-500/10 text-green-400',
    yellow: 'bg-yellow-500/10 text-yellow-400',
    red: 'bg-red-500/10 text-red-400',
    purple: 'bg-purple-500/10 text-purple-400',
  }
</script>

<div class="stat-card">
  <div class="stat-icon" class:blue={color === 'blue'} class:green={color === 'green'} class:yellow={color === 'yellow'} class:red={color === 'red'} class:purple={color === 'purple'}>
    {icon}
  </div>
  <div class="stat-content">
    <div class="stat-label">{label}</div>
    <div class="stat-value">{value}</div>
    {#if trend !== null}
      <div class="stat-trend" class:positive={trend >= 0} class:negative={trend < 0}>
        {trend >= 0 ? '↑' : '↓'} {Math.abs(trend)}%
      </div>
    {/if}
  </div>
</div>

<style>
  .stat-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 20px;
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .stat-icon {
    width: 56px;
    height: 56px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 28px;
  }
  .stat-icon.blue { background: rgba(59, 130, 246, 0.1); }
  .stat-icon.green { background: rgba(34, 197, 94, 0.1); }
  .stat-icon.yellow { background: rgba(234, 179, 8, 0.1); }
  .stat-icon.red { background: rgba(239, 68, 68, 0.1); }
  .stat-icon.purple { background: rgba(168, 85, 247, 0.1); }
  .stat-content {
    flex: 1;
  }
  .stat-label {
    font-size: 14px;
    color: #94a3b8;
    margin-bottom: 4px;
  }
  .stat-value {
    font-size: 28px;
    font-weight: 700;
    color: #f1f5f9;
  }
  .stat-trend {
    font-size: 13px;
    font-weight: 500;
    margin-top: 4px;
  }
  .stat-trend.positive { color: #22c55e; }
  .stat-trend.negative { color: #ef4444; }
</style>
