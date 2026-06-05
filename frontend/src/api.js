import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 30000,
})

export const healthCheck = () => api.get('/health')

export const getTraces = (params = {}) => api.get('/traces', { params })
export const getTrace = (id) => api.get(`/traces/${id}`)
export const getTraceSpans = (id) => api.get(`/traces/${id}/spans`)
export const getCriticalPath = (traceId) => api.get(`/traces/${traceId}/critical-path`)
export const compareTraces = (baseline, comparison) => 
  api.get('/traces/compare', { params: { baseline, comparison } })

export const getServices = () => api.get('/services')
export const getServiceDetails = (name) => api.get(`/services/${name}`)
export const getServiceMetrics = (name, params = {}) => 
  api.get(`/services/${name}/metrics`, { params })

export const getTopology = () => api.get('/topology')

export const getLatencyDistribution = (params = {}) => 
  api.get('/analysis/latency-distribution', { params })
export const getAnomalies = (params = {}) => 
  api.get('/analysis/anomalies', { params })

export const getSLOs = () => api.get('/slo')
export const createSLO = (data) => api.post('/slo', data)
export const getSLO = (id) => api.get(`/slo/${id}`)
export const updateSLO = (id, data) => api.put(`/slo/${id}`, data)
export const deleteSLO = (id) => api.delete(`/slo/${id}`)
export const getSLOStatus = (id) => api.get(`/slo/${id}/status`)

export const uploadTraceFile = (file, format = 'otel') => {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('format', format)
  return api.post('/import/upload', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}

export const pushSpans = (data, format = 'otel') => 
  api.post('/import/push', data, { params: { format } })

export const getImportJobs = () => api.get('/import/jobs')
export const getImportJob = (id) => api.get(`/import/jobs/${id}`)
export const getImportProgress = (id) => api.get(`/import/jobs/${id}/progress`)

export default api
