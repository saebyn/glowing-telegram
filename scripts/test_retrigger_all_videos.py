"""
Tests for scripts/retrigger_all_videos.py
"""
import pytest
import os


class TestPythonFileExists:
    """Basic test to ensure Python file exists and has valid syntax"""
    
    def test_retrigger_all_videos_py_exists(self):
        """Test that retrigger_all_videos.py exists"""
        script_path = os.path.join(os.path.dirname(__file__), 'retrigger_all_videos.py')
        assert os.path.exists(script_path), "retrigger_all_videos.py does not exist"
    
    def test_retrigger_all_videos_py_syntax(self):
        """Test that retrigger_all_videos.py has valid syntax"""
        import ast
        
        script_path = os.path.join(os.path.dirname(__file__), 'retrigger_all_videos.py')
        if os.path.exists(script_path):
            with open(script_path, 'r') as f:
                try:
                    ast.parse(f.read())
                except SyntaxError:
                    pytest.fail("Syntax error in retrigger_all_videos.py")