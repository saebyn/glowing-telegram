// Mock AWS SDK first
jest.mock('aws-sdk');

const AWS = require('aws-sdk');

describe('Version Selector Lambda@Edge', () => {
  let handler, resetCache;
  let mockS3GetObject;

  beforeEach(() => {
    // Set environment variables before requiring the module
    process.env.BUCKET_NAME = 'test-bucket';
    
    // Clear the module cache and re-require to pick up environment variables
    delete require.cache[require.resolve('./index')];
    const module = require('./index');
    handler = module.handler;
    resetCache = module.resetCache;
    
    // Reset cache before each test
    resetCache();
    
    // Mock S3
    mockS3GetObject = jest.fn();
    AWS.S3 = jest.fn(() => ({
      getObject: mockS3GetObject
    }));
  });

  afterEach(() => {
    jest.clearAllMocks();
    delete process.env.BUCKET_NAME;
  });

  test('should rewrite URI with version from S3 config', async () => {
    // Mock S3 response
    const mockVersionConfig = {
      version: '1.2.3'
    };
    
    mockS3GetObject.mockReturnValue({
      promise: () => Promise.resolve({
        Body: {
          toString: () => JSON.stringify(mockVersionConfig)
        }
      })
    });

    // Mock CloudFront event
    const event = {
      Records: [{
        cf: {
          request: {
            uri: '/index.html'
          }
        }
      }]
    };

    // Execute handler
    const result = await handler(event);

    // Verify the URI was rewritten correctly
    expect(result.uri).toBe('/1.2.3/index.html');
    
    // Verify S3 was called with correct parameters
    expect(mockS3GetObject).toHaveBeenCalledWith({
      Bucket: 'test-bucket',
      Key: 'config/version.json'
    });
  });

  test('should handle root path correctly', async () => {
    // Reset cache to ensure fresh S3 call
    resetCache();
    
    const mockVersionConfig = {
      version: '2.0.0'
    };
    
    mockS3GetObject.mockReturnValue({
      promise: () => Promise.resolve({
        Body: {
          toString: () => JSON.stringify(mockVersionConfig)
        }
      })
    });

    const event = {
      Records: [{
        cf: {
          request: {
            uri: '/'
          }
        }
      }]
    };

    const result = await handler(event);
    expect(result.uri).toBe('/2.0.0/index.html');
  });

  test('should return original request on S3 error', async () => {
    // Reset cache to ensure fresh S3 call
    resetCache();
    
    // Mock S3 error
    mockS3GetObject.mockReturnValue({
      promise: () => Promise.reject(new Error('S3 Error'))
    });

    const event = {
      Records: [{
        cf: {
          request: {
            uri: '/test.js'
          }
        }
      }]
    };

    const result = await handler(event);
    
    // Should return original request unchanged
    expect(result.uri).toBe('/test.js');
  });

  test('should not duplicate version in URI', async () => {
    // Reset cache to ensure fresh S3 call
    resetCache();
    
    const mockVersionConfig = {
      version: '1.0.0'
    };
    
    mockS3GetObject.mockReturnValue({
      promise: () => Promise.resolve({
        Body: {
          toString: () => JSON.stringify(mockVersionConfig)
        }
      })
    });

    const event = {
      Records: [{
        cf: {
          request: {
            uri: '/1.0.0/style.css'
          }
        }
      }]
    };

    const result = await handler(event);
    
    // Should not duplicate the version
    expect(result.uri).toBe('/1.0.0/style.css');
  });
});