"""
Tests for audio_transcriber/download_model.py
"""
import pytest
import os


class TestPythonFileExists:
    """Basic test to ensure Python file exists and has valid syntax"""
    
    def test_download_model_py_exists(self):
        """Test that download_model.py exists"""
        script_path = os.path.join(os.path.dirname(__file__), 'download_model.py')
        assert os.path.exists(script_path), "download_model.py does not exist"
    
    def test_download_model_py_syntax(self):
        """Test that download_model.py has valid syntax"""
        import ast
        
        script_path = os.path.join(os.path.dirname(__file__), 'download_model.py')
        if os.path.exists(script_path):
            with open(script_path, 'r') as f:
                try:
                    ast.parse(f.read())
                except SyntaxError:
                    pytest.fail("Syntax error in download_model.py")