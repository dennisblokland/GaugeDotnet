using System.Collections.Generic;

namespace GaugeDotnet.Gauges.Custom;

internal static class ExpressionEvaluator
{
	public static float Evaluate(string expression, Dictionary<string, float> values)
	{
		ExprParser p = new(expression, values);
		return p.ParseExpr();
	}

	private sealed class ExprParser(string src, Dictionary<string, float> values)
	{
		private readonly string _src = src;
		private readonly Dictionary<string, float> _values = values;
		private int _pos;

		public float ParseExpr() => ParseAdd();

		private float ParseAdd()
		{
			float left = ParseMul();
			while (true)
			{
				SkipWs();
				if (_pos < _src.Length && _src[_pos] == '+') { _pos++; left += ParseMul(); }
				else if (_pos < _src.Length && _src[_pos] == '-') { _pos++; left -= ParseMul(); }
				else break;
			}
			return left;
		}

		private float ParseMul()
		{
			float left = ParseUnary();
			while (true)
			{
				SkipWs();
				if (_pos < _src.Length && _src[_pos] == '*') { _pos++; left *= ParseUnary(); }
				else if (_pos < _src.Length && _src[_pos] == '/')
				{
					_pos++;
					float r = ParseUnary();
					left = r != 0 ? left / r : 0;
				}
				else break;
			}
			return left;
		}

		private float ParseUnary()
		{
			SkipWs();
			if (_pos < _src.Length && _src[_pos] == '-') { _pos++; return -ParsePrimary(); }
			if (_pos < _src.Length && _src[_pos] == '+') { _pos++; return ParsePrimary(); }
			return ParsePrimary();
		}

		private float ParsePrimary()
		{
			SkipWs();
			if (_pos >= _src.Length) return 0;

			if (_src[_pos] == '(')
			{
				_pos++;
				float v = ParseExpr();
				SkipWs();
				if (_pos < _src.Length && _src[_pos] == ')') _pos++;
				return v;
			}

			if (char.IsDigit(_src[_pos]) || _src[_pos] == '.')
			{
				int start = _pos;
				while (_pos < _src.Length && (char.IsDigit(_src[_pos]) || _src[_pos] == '.')) _pos++;
				return float.TryParse(_src.AsSpan(start, _pos - start), out float n) ? n : 0;
			}

			if (char.IsLetter(_src[_pos]) || _src[_pos] == '_')
			{
				int start = _pos;
				while (_pos < _src.Length && (char.IsLetterOrDigit(_src[_pos]) || _src[_pos] == '_')) _pos++;
				string name = _src[start.._pos];
				return _values.TryGetValue(name, out float val) ? val : 0;
			}

			_pos++;
			return 0;
		}

		private void SkipWs()
		{
			while (_pos < _src.Length && char.IsWhiteSpace(_src[_pos])) _pos++;
		}
	}
}
