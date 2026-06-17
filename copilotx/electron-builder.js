const config = require('./config/config.json')

module.exports = {
  appId: 'com.copilotx',
  productName: 'CopilotX',
  directories: {
    buildResources: 'build',
    output: 'dist'
  },
  files: [
    'out/**/*',
    'resources/**/*',
    '!**/.vscode/*',
    '!src/*',
    '!sidecar/*',
    '!{.eslintcache,.prettierrc.yaml,dev-app-update.yml}',
    '!{.env,.env.*,.npmrc,pnpm-lock.yaml}',
    '!{tsconfig*.json,electron.vite.config.*}'
  ],
  win: {
    executableName: config.processName || 'CopilotX',
    target: ['zip'],
    extraResources: [
      {
        from: `resources/${config.sidecarName || 'svchost'}.exe`,
        to: `${config.sidecarName || 'svchost'}.exe`
      },
      {
        from: 'config/config.json',
        to: 'config.json'
      }
    ]
  },
  npmRebuild: false,
  linux: {
    executableName: 'copilotx',
    target: ['AppImage', 'deb'],
    category: 'Utility',
    maintainer: 'CopilotX Team',
    extraResources: [
      {
        from: `resources/${config.sidecarName || 'svchost'}`,
        to: `${config.sidecarName || 'svchost'}`
      },
      {
        from: 'config/config.json',
        to: 'config.json'
      }
    ]
  }
}
