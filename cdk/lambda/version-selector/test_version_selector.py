#!/usr/bin/env python3
"""
Test suite for the Lambda@Edge version selector function.
Tests the Python implementation.
"""

import json
import unittest
from unittest.mock import Mock, patch, MagicMock
import sys
import os

# Import the actual function
import index


class TestVersionSelector(unittest.TestCase):
    """Test cases for the version selector Lambda@Edge function."""

    def setUp(self):
        """Set up test fixtures."""
        self.test_bucket = 'test-bucket'
        self.fallback_version = '0.4.0'
        
        # Set environment variables for testing
        os.environ['BUCKET_NAME'] = self.test_bucket
        os.environ['FALLBACK_VERSION'] = self.fallback_version
        
        # Reset cache before each test
        index.reset_cache()
        
    def create_cloudfront_event(self, uri):
        """Create a mock CloudFront event."""
        return {
            'Records': [{
                'cf': {
                    'request': {
                        'uri': uri
                    }
                }
            }]
        }
    
    @patch('index.get_current_version')
    def test_uri_rewriting_with_s3_version(self, mock_get_version):
        """Test that URI is rewritten with version from S3 config."""
        mock_get_version.return_value = '1.2.3'
        
        event = self.create_cloudfront_event('/index.html')
        result = index.handler(event, {})
        
        self.assertEqual(result['uri'], '/1.2.3/index.html')
    
    @patch('index.get_current_version')
    def test_root_path_handling(self, mock_get_version):
        """Test that root path is handled correctly."""
        mock_get_version.return_value = '2.0.0'
        
        event = self.create_cloudfront_event('/')
        result = index.handler(event, {})
        
        self.assertEqual(result['uri'], '/2.0.0/index.html')
    
    @patch('index.get_current_version')
    def test_empty_uri_handling(self, mock_get_version):
        """Test handling of empty URI."""
        mock_get_version.return_value = '1.0.0'
        
        event = self.create_cloudfront_event('')
        result = index.handler(event, {})
        
        self.assertEqual(result['uri'], '/1.0.0/index.html')
    
    @patch('index.get_current_version')
    def test_fallback_to_original_version(self, mock_get_version):
        """Test fallback behavior when S3 fails."""
        mock_get_version.return_value = None  # Simulate S3 failure
        
        event = self.create_cloudfront_event('/test.js')
        result = index.handler(event, {})
        
        self.assertEqual(result['uri'], f'/{self.fallback_version}/test.js')
    
    @patch('index.get_current_version')
    def test_fallback_root_path(self, mock_get_version):
        """Test fallback behavior for root path."""
        mock_get_version.return_value = None  # Simulate S3 failure
        
        event = self.create_cloudfront_event('/')
        result = index.handler(event, {})
        
        self.assertEqual(result['uri'], f'/{self.fallback_version}/index.html')
    
    @patch('index.get_current_version')
    def test_no_version_available(self, mock_get_version):
        """Test behavior when no version is available at all."""
        mock_get_version.return_value = None
        # Remove fallback version
        del os.environ['FALLBACK_VERSION']
        
        event = self.create_cloudfront_event('/test.html')
        result = index.handler(event, {})
        
        # Should return original request unchanged
        self.assertEqual(result['uri'], '/test.html')
        
        # Restore for other tests
        os.environ['FALLBACK_VERSION'] = self.fallback_version
    
    @patch('index.get_current_version')
    def test_error_handling_with_exception(self, mock_get_version):
        """Test error handling when get_current_version raises exception."""
        mock_get_version.side_effect = Exception("S3 connection error")
        
        event = self.create_cloudfront_event('/app.js')
        result = index.handler(event, {})
        
        # Should fallback to FALLBACK_VERSION
        self.assertEqual(result['uri'], f'/{self.fallback_version}/app.js')

class TestGetCurrentVersion(unittest.TestCase):
    """Test the get_current_version function."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.test_bucket = 'test-bucket'
        os.environ['BUCKET_NAME'] = self.test_bucket
        index.reset_cache()
    
    @patch('boto3.client')
    def test_s3_version_fetching(self, mock_boto_client):
        """Test fetching version from S3."""
        mock_s3 = Mock()
        mock_boto_client.return_value = mock_s3
        
        # Mock S3 response
        mock_s3.get_object.return_value = {
            'Body': Mock(read=Mock(return_value=json.dumps({'version': '1.5.0'}).encode('utf-8')))
        }
        
        version = index.get_current_version()
        
        self.assertEqual(version, '1.5.0')
        mock_s3.get_object.assert_called_once_with(Bucket=self.test_bucket, Key='config/version.json')
    
    @patch('boto3.client')
    def test_caching_behavior(self, mock_boto_client):
        """Test that caching works properly."""
        mock_s3 = Mock()
        mock_boto_client.return_value = mock_s3
        
        # Mock S3 response
        mock_s3.get_object.return_value = {
            'Body': Mock(read=Mock(return_value=json.dumps({'version': '2.0.0'}).encode('utf-8')))
        }
        
        # First call should fetch from S3
        version1 = index.get_current_version()
        
        # Second call should use cache
        version2 = index.get_current_version()
        
        self.assertEqual(version1, '2.0.0')
        self.assertEqual(version2, '2.0.0')
        # S3 should only be called once due to caching
        mock_s3.get_object.assert_called_once()
    
    @patch('boto3.client')
    def test_s3_error_handling(self, mock_boto_client):
        """Test error handling when S3 fails."""
        mock_s3 = Mock()
        mock_boto_client.return_value = mock_s3
        
        # Mock S3 error
        from botocore.exceptions import ClientError
        mock_s3.get_object.side_effect = ClientError(
            error_response={'Error': {'Code': 'NoSuchKey'}},
            operation_name='GetObject'
        )
        
        version = index.get_current_version()
        
        self.assertIsNone(version)
    
    @patch('boto3.client')
    def test_invalid_json_handling(self, mock_boto_client):
        """Test handling of invalid JSON in S3."""
        mock_s3 = Mock()
        mock_boto_client.return_value = mock_s3
        
        # Mock S3 response with invalid JSON
        mock_s3.get_object.return_value = {
            'Body': Mock(read=Mock(return_value=b'invalid json'))
        }
        
        version = index.get_current_version()
        
        self.assertIsNone(version)


if __name__ == '__main__':
    print("Running Lambda@Edge Version Selector Test Suite")
    print("=" * 50)
    
    # Run the tests
    unittest.main(verbosity=2)