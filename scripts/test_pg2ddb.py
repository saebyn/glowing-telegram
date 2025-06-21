"""
Tests for scripts/pg2ddb.py
"""
import pytest
import os


class TestPythonFileExists:
    """Basic test to ensure Python file exists and has valid syntax"""
    
    def test_pg2ddb_py_exists(self):
        """Test that pg2ddb.py exists"""
        script_path = os.path.join(os.path.dirname(__file__), 'pg2ddb.py')
        assert os.path.exists(script_path), "pg2ddb.py does not exist"
    
    def test_pg2ddb_py_syntax(self):
        """Test that pg2ddb.py has valid syntax"""
        import ast
        
        script_path = os.path.join(os.path.dirname(__file__), 'pg2ddb.py')
        if os.path.exists(script_path):
            with open(script_path, 'r') as f:
                try:
                    ast.parse(f.read())
                except SyntaxError:
                    pytest.fail("Syntax error in pg2ddb.py")