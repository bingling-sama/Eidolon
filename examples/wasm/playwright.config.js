import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:4173',
    headless: true,
  },
  projects: [
    {
      name: 'webgpu',
      testMatch: /.*\.spec\.js/,
      use: {
        launchOptions: {
          args: [
            '--enable-unsafe-webgpu',
            '--enable-features=Vulkan',
            '--use-angle=swiftshader',
            '--use-vulkan=swiftshader',
          ],
        },
      },
    },
    {
      name: 'webgl2',
      testMatch: /.*\.spec\.js/,
      use: {
        launchOptions: {
          args: [
            '--disable-webgpu',
            '--use-gl=angle',
            '--use-angle=swiftshader',
          ],
        },
      },
    },
  ],
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 4173 --strictPort',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: true,
  },
});
