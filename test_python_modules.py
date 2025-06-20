"""
Basic tests for Python modules in the glowing-telegram repository.
"""
import pytest
import sys
import os
import urllib.parse
from unittest.mock import patch, MagicMock


class TestMediaLambda:
    """Tests for media_lambda/main.py"""
    
    @patch.dict(os.environ, {
        'VIDEO_METADATA_TABLE': 'test_table',
        'STREAM_ID_INDEX': 'test_index'
    })
    def test_rewrite_path(self):
        """Test the rewrite_path function"""
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'media_lambda'))
        import main as media_main
        
        # Test basic path rewriting
        result = media_main.rewrite_path("transcode/test/file.mp4")
        assert result == "/test/file.mp4"
        
        # Test URL encoding
        result = media_main.rewrite_path("transcode/test file with spaces.mp4")
        assert "file%20with%20spaces" in result
        
        # Clean up import
        if 'media_lambda.main' in sys.modules:
            del sys.modules['media_lambda.main']
        if 'main' in sys.modules:
            del sys.modules['main']


class TestWebSocketAuthorizer:
    """Tests for websocket authorizer"""
    
    @patch.dict(os.environ, {
        'USER_POOL_ID': 'test_pool_id',
        'USER_POOL_CLIENT_ID': 'test_client_id',
        'AWS_REGION': 'us-west-2'
    })
    def test_generate_policy(self):
        """Test the generate_policy function"""
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'cdk', 'lib', 'websocketAuthorizer'))
        import main as auth_main
        
        policy = auth_main.generate_policy("user123", "Allow", "arn:aws:execute-api:*")
        
        assert policy["principalId"] == "user123"
        assert policy["policyDocument"]["Statement"][0]["Effect"] == "Allow"
        assert policy["policyDocument"]["Statement"][0]["Resource"] == "arn:aws:execute-api:*"
        
        # Test with context
        policy_with_context = auth_main.generate_policy("user123", "Allow", "arn:aws:execute-api:*", {"key": "value"})
        assert policy_with_context["context"]["key"] == "value"


class TestPythonFilesExist:
    """Basic tests to ensure Python files exist"""
    
    def test_python_files_exist(self):
        """Test that Python files exist"""
        files_to_check = [
            'media_lambda/main.py',
            'cdk/lib/websocketAuthorizer/main.py',
            'scripts/pg2ddb.py',
            'scripts/retrigger_all_videos.py',
            'audio_transcriber/download_model.py'
        ]
        
        for file_path in files_to_check:
            full_path = os.path.join(os.path.dirname(__file__), file_path)
            assert os.path.exists(full_path), f"File {file_path} does not exist"
    
    def test_basic_python_syntax(self):
        """Test that Python files have valid syntax"""
        import ast
        
        files_to_check = [
            'media_lambda/main.py',
            'cdk/lib/websocketAuthorizer/main.py',
            'scripts/retrigger_all_videos.py',
            'audio_transcriber/download_model.py'
        ]
        
        for file_path in files_to_check:
            full_path = os.path.join(os.path.dirname(__file__), file_path)
            if os.path.exists(full_path):
                with open(full_path, 'r') as f:
                    try:
                        ast.parse(f.read())
                    except SyntaxError:
                        pytest.fail(f"Syntax error in {file_path}")