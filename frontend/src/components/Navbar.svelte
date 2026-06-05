<script>
  import { Link, location } from 'svelte-routing'

  const navItems = [
    { path: '/', label: 'Dashboard', icon: '📊' },
    { path: '/topology', label: '服务拓扑', icon: '🕸️' },
    { path: '/traces', label: 'Trace搜索', icon: '🔍' },
    { path: '/analysis', label: '性能分析', icon: '📈' },
    { path: '/slo', label: 'SLO监控', icon: '🎯' },
    { path: '/alerts/rules', label: '告警中心', icon: '🔔' },
    { path: '/import', label: '数据导入', icon: '📤' },
  ]
</script>

<nav class="navbar">
  <div class="nav-brand">
    <span class="logo">🔬</span>
    <span class="title">Trace Diagnose</span>
  </div>
  <div class="nav-links">
    {#each navItems as item}
      <Link to={item.path} class="nav-link" class:active={$location.pathname === item.path}>
        <span class="icon">{item.icon}</span>
        <span class="label">{item.label}</span>
      </Link>
    {/each}
  </div>
</nav>

<style>
  .navbar {
    background: #1e293b;
    border-bottom: 1px solid #334155;
    padding: 0 24px;
    display: flex;
    align-items: center;
    gap: 48px;
    height: 64px;
  }
  .nav-brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .logo {
    font-size: 28px;
  }
  .title {
    font-size: 20px;
    font-weight: 600;
    color: #f1f5f9;
  }
  .nav-links {
    display: flex;
    gap: 8px;
    flex: 1;
  }
  .nav-link {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    text-decoration: none;
    color: #94a3b8;
    transition: all 0.2s;
  }
  .nav-link:hover {
    background: #334155;
    color: #f1f5f9;
  }
  .nav-link.active {
    background: #3b82f6;
    color: white;
  }
  .icon {
    font-size: 18px;
  }
  .label {
    font-size: 14px;
    font-weight: 500;
  }
</style>
