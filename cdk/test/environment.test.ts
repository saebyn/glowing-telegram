import { 
  loadEnvironmentConfig, 
  getStackName, 
  getResourceName,
  getRoleName,
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

  test('loads unit-testing environment by default', () => {
    delete process.env.ENVIRONMENT;
    const { name, config } = loadEnvironmentConfig();
    
    expect(name).toBe('unit-testing');
    expect(config.awsAccount).toBe(null);
    expect(config.awsRegion).toBe('us-west-2');
    expect(config.tags.Environment).toBe('unit-testing');
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
    const { name, config } = loadEnvironmentConfig('dev');
    
    expect(name).toBe('dev');
    expect(config.awsAccount).toBe('159222827421');
    expect(config.awsRegion).toBe('us-west-2');
    expect(config.tags.Environment).toBe('dev');
    expect(config.twitchClientId).toBeDefined();
    expect(config.githubOwner).toBeDefined();
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
    expect(getStackName('FrontendStack', 'dev')).toBe('FrontendStack-dev');
  });

  test('getResourceName returns base name for production', () => {
    expect(getResourceName('my-bucket', 'production')).toBe('my-bucket');
    expect(getResourceName('my-role', 'production')).toBe('my-role');
  });

  test('getResourceName returns prefixed name for non-production', () => {
    expect(getResourceName('my-bucket', 'dev')).toBe('dev-my-bucket');
    expect(getResourceName('my-role', 'dev')).toBe('dev-my-role');
  });

  test('getRoleName returns undefined for production', () => {
    expect(getRoleName('MyRole', 'production')).toBeUndefined();
  });

  test('getRoleName returns suffixed name for non-production', () => {
    expect(getRoleName('MyRole', 'dev')).toBe('MyRole-dev');
  });

  test('getAvailableEnvironments returns all environments', () => {
    const environments = getAvailableEnvironments();
    
    expect(environments).toContain('production');
    expect(environments).toContain('dev');
    expect(environments).toContain('unit-testing');
    expect(environments.length).toBe(3);
  });
});
