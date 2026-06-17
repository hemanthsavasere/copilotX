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
    executableName: 'CopilotX',
    target: ['zip'],
    extraResources: [
      {
        from: 'resources/system-helper.exe',
        to: 'system-helper.exe'
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
        from: 'resources/system-helper',
        to: 'system-helper'
      },
      {
        from: 'config/config.json',
        to: 'config.json'
      }
    ]
  }
}
