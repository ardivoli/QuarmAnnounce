import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface MessageConfig {
  type: 'simple' | 'timed_delay'
  pattern: string
  announcement: string
  timer_delay_in_seconds?: number
}

interface Config {
  game_directory: string
  messages: MessageConfig[]
}

function App() {
  const [config, setConfig] = useState<Config | null>(null)
  const [isMonitoring, setIsMonitoring] = useState(false)
  const [status, setStatus] = useState('Idle')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      setStatus('Loading configuration...')
      setError(null)
      const cfg = await invoke<Config>('load_config', {
        path: './config.json',
      })
      setConfig(cfg)
      setStatus('Configuration loaded')
    } catch (e) {
      const errorMsg = `Failed to load config: ${e}`
      setError(errorMsg)
      setStatus('Error')
      console.error(errorMsg)
    }
  }

  const toggleMonitoring = async () => {
    try {
      if (!isMonitoring) {
        setStatus('Initializing TTS...')
        setError(null)

        // Initialize TTS engine
        await invoke('init_tts', {
          modelPath: './resources/speakers/en_US-amy-medium.onnx.json',
        })

        // Start monitoring
        setStatus('Starting monitoring...')
        await invoke('start_monitoring')
        setIsMonitoring(true)
        setStatus('Monitoring active')
      } else {
        setStatus('Stopping monitoring...')
        await invoke('stop_monitoring')
        setIsMonitoring(false)
        setStatus('Monitoring stopped')
      }
    } catch (e) {
      const errorMsg = `Failed to ${isMonitoring ? 'stop' : 'start'} monitoring: ${e}`
      setError(errorMsg)
      setStatus('Error')
      console.error(errorMsg)
    }
  }

  const testAnnouncement = async (text: string) => {
    try {
      setError(null)
      setStatus(`Testing announcement: "${text}"...`)
      await invoke('test_announcement', { text })
      setStatus('Test complete')
    } catch (e) {
      const errorMsg = `Failed to test announcement: ${e}`
      setError(errorMsg)
      setStatus('Error')
      console.error(errorMsg)
    }
  }

  return (
    <div style={{ padding: '20px', fontFamily: 'system-ui, sans-serif' }}>
      <h1>Quarm Announce</h1>

      {/* Status Section */}
      <div style={{ marginBottom: '20px', padding: '10px', backgroundColor: '#f0f0f0', borderRadius: '4px' }}>
        <strong>Status:</strong> {status}
        {error && <div style={{ color: 'red', marginTop: '5px' }}>{error}</div>}
      </div>

      {/* Monitoring Control */}
      <div style={{ marginBottom: '20px' }}>
        <button
          onClick={toggleMonitoring}
          disabled={!config}
          style={{
            padding: '10px 20px',
            fontSize: '16px',
            backgroundColor: isMonitoring ? '#dc3545' : '#28a745',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: config ? 'pointer' : 'not-allowed',
          }}
        >
          {isMonitoring ? 'Stop Monitoring' : 'Start Monitoring'}
        </button>
      </div>

      {/* Configuration Display */}
      {config ? (
        <div>
          <h2>Configuration</h2>
          <div style={{ marginBottom: '10px' }}>
            <strong>Game Directory:</strong> {config.game_directory}
          </div>

          <h3>Message Patterns ({config.messages.length})</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {config.messages.map((msg, idx) => (
              <div
                key={idx}
                style={{
                  padding: '10px',
                  border: '1px solid #ddd',
                  borderRadius: '4px',
                  backgroundColor: '#fafafa',
                }}
              >
                <div style={{ marginBottom: '5px' }}>
                  <strong>Type:</strong> {msg.type}
                  {msg.type === 'timed_delay' && msg.timer_delay_in_seconds && (
                    <span style={{ marginLeft: '10px', color: '#666' }}>
                      (Delay: {msg.timer_delay_in_seconds}s)
                    </span>
                  )}
                </div>
                <div style={{ marginBottom: '5px' }}>
                  <strong>Pattern:</strong> {msg.pattern}
                </div>
                <div style={{ marginBottom: '10px' }}>
                  <strong>Announcement:</strong> {msg.announcement}
                </div>
                <button
                  onClick={() => testAnnouncement(msg.announcement)}
                  disabled={isMonitoring}
                  style={{
                    padding: '5px 10px',
                    fontSize: '14px',
                    backgroundColor: isMonitoring ? '#ccc' : '#007bff',
                    color: 'white',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: isMonitoring ? 'not-allowed' : 'pointer',
                  }}
                >
                  Test Announcement
                </button>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div>
          <p>No configuration loaded.</p>
          <button
            onClick={loadConfig}
            style={{
              padding: '10px 20px',
              fontSize: '14px',
              backgroundColor: '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
            }}
          >
            Retry Load Config
          </button>
        </div>
      )}
    </div>
  )
}

export default App
