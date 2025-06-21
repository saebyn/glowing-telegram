"""
Tests for media_lambda/main.py
"""
import pytest
import sys
import os
import urllib.parse
import importlib.util
from unittest.mock import patch, MagicMock


class TestMediaLambda:
    """Tests for media_lambda/main.py"""
    
    @patch.dict(os.environ, {
        'VIDEO_METADATA_TABLE': 'test_table',
        'STREAM_ID_INDEX': 'test_index'
    })
    def test_rewrite_path(self):
        """Test the rewrite_path function"""
        # Import the specific main.py file using importlib
        spec = importlib.util.spec_from_file_location(
            "media_lambda_main", 
            os.path.join(os.path.dirname(__file__), "main.py")
        )
        media_main = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(media_main)
        
        # Test basic path rewriting
        result = media_main.rewrite_path("transcode/test/file.mp4")
        assert result == "/test/file.mp4"
        
        # Test URL encoding
        result = media_main.rewrite_path("transcode/test file with spaces.mp4")
        assert "file%20with%20spaces" in result


class TestPythonFileExists:
    """Basic test to ensure Python file exists and has valid syntax"""
    
    def test_main_py_exists(self):
        """Test that main.py exists"""
        main_path = os.path.join(os.path.dirname(__file__), 'main.py')
        assert os.path.exists(main_path), "main.py does not exist"
    
    def test_main_py_syntax(self):
        """Test that main.py has valid syntax"""
        import ast
        
        main_path = os.path.join(os.path.dirname(__file__), 'main.py')
        if os.path.exists(main_path):
            with open(main_path, 'r') as f:
                try:
                    ast.parse(f.read())
                except SyntaxError:
                    pytest.fail("Syntax error in main.py")