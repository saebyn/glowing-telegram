import { 
  loadEnvironmentConfig, 
  getStackName, 
  getResourceName,
  getAvailableEnvironments 
} from '../lib/util/environment';

describe('Environment Configuration', () => {
  // Store original env
  const originalEnv = process.env.ENVIRONMENT;

  afterEach(() => {
    // Restore original env
    if (originalEnv) {
      process.env.ENVIRONMENT = originalEnv;
    } else {
      delete process.env.ENVIRONMENT;
    }
  });

  test('loads production environment by default', () => {
    delete process.env.ENVIRONMENT;
    const { name, config } = loadEnvironmentConfig();
    
    expect(name).toBe('production');
    expect(config.awsAccount).toBe('159222827421');
    expect(config.awsRegion).toBe('us-west-2');
    expect(config.tags.Environment).toBe('production');
  });

  test('loads environment from ENVIRONMENT variable', () => {
    process.env.ENVIRONMENT = 'dev';
    const { name, config } = loadEnvironmentConfig();
    
    expect(name).toBe('dev');
    expect(config.awsAccount).toBe('159222827421');
    expect(config.awsRegion).toBe('us-west-2');
    expect(config.tags.Environment).toBe('dev');
  });

  test('loads environment from parameter', () => {
    const { name, config } = loadEnvironmentConfig('staging');
    
    expect(name).toBe('staging');
    expect(config.awsAccount).toBe('159222827421');
    expect(config.awsRegion).toBe('us-west-2');
    expect(config.tags.Environment).toBe('staging');
  });

  test('throws error for invalid environment', () => {
    expect(() => {
      loadEnvironmentConfig('invalid-env');
    }).toThrow(/Environment "invalid-env" not found/);
  });

  test('getStackName returns base name for production', () => {
    expect(getStackName('AppStack', 'production')).toBe('AppStack');
    expect(getStackName('FrontendStack', 'production')).toBe('FrontendStack');
  });

  test('getStackName returns suffixed name for non-production', () => {
    expect(getStackName('AppStack', 'dev')).toBe('AppStack-dev');
    expect(getStackName('FrontendStack', 'staging')).toBe('FrontendStack-staging');
  });

  test('getResourceName returns base name for production', () => {
    expect(getResourceName('my-bucket', 'production')).toBe('my-bucket');
    expect(getResourceName('my-role', 'production')).toBe('my-role');
  });

  test('getResourceName returns prefixed name for non-production', () => {
    expect(getResourceName('my-bucket', 'dev')).toBe('dev-my-bucket');
    expect(getResourceName('my-role', 'staging')).toBe('staging-my-role');
  });

  test('getAvailableEnvironments returns all environments', () => {
    const environments = getAvailableEnvironments();
    
    expect(environments).toContain('production');
    expect(environments).toContain('staging');
    expect(environments).toContain('dev');
    expect(environments.length).toBeGreaterThanOrEqual(3);
  });
});
