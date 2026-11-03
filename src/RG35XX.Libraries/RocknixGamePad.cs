using RG35XX.Core.GamePads;

namespace RG35XX.Libraries;

/// <summary>
/// Maps ROCKNIX's "H700 Gamepad" joystick events to the codes <see cref="GamepadKey"/>
/// was written for on muOS. The D-pad is buttons 13-16 there, and the sticks are axes
/// whose numbers overlap the buttons. Same table as rust/gauge-app/src/input.rs.
/// </summary>
internal static class RocknixGamePad
{
	private const byte JsEventButton = 0x01;
	private const byte JsEventInit = 0x80;
	private const GamepadKey MenuRelease = (GamepadKey)524288;

	public static GamepadKey Translate(byte type, byte number, short value)
	{
		if ((type & JsEventInit) != 0 || type != JsEventButton)
		{
			return GamepadKey.None;
		}

		(GamepadKey down, GamepadKey up) = number switch
		{
			0 => (GamepadKey.B_DOWN, GamepadKey.B_UP),
			1 => (GamepadKey.A_DOWN, GamepadKey.A_UP),
			2 => (GamepadKey.Y_DOWN, GamepadKey.Y_UP),
			3 => (GamepadKey.X_DOWN, GamepadKey.X_UP),
			4 => (GamepadKey.L1_DOWN, GamepadKey.L1_UP),
			5 => (GamepadKey.R1_DOWN, GamepadKey.R1_UP),
			6 => (GamepadKey.L2_DOWN, GamepadKey.L2_UP),
			7 => (GamepadKey.R2_DOWN, GamepadKey.R2_UP),
			8 => (GamepadKey.SELECT_DOWN, GamepadKey.SELECT_UP),
			9 => (GamepadKey.START_DOWN, GamepadKey.START_UP),
			10 => (GamepadKey.MENU_DOWN, MenuRelease),
			13 => (GamepadKey.UP, GamepadKey.UP_DOWN_UP),
			14 => (GamepadKey.DOWN, GamepadKey.UP_DOWN_UP),
			15 => (GamepadKey.LEFT, GamepadKey.LEFT_RIGHT_UP),
			16 => (GamepadKey.RIGHT, GamepadKey.LEFT_RIGHT_UP),
			_ => (GamepadKey.None, GamepadKey.None),
		};
		return value != 0 ? down : up;
	}
}
