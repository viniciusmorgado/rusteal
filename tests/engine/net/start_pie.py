import os

import unreal

RESULTS = os.environ.get("RUSTEAL_NET_TEST_RESULTS")


def quit_editor():
    unreal.SystemLibrary.execute_console_command(None, "QUIT_EDITOR")


def fail(reason):
    if RESULTS:
        with open(RESULTS, "w") as results:
            results.write(f"FAIL net_play_in_editor: {reason}\nDONE\n")

    quit_editor()


def finished():
    try:
        with open(RESULTS) as results:
            return "DONE" in results.read().splitlines()
    except OSError:
        return False


def quit_when_finished(_delta_seconds):
    if finished():
        unreal.unregister_slate_post_tick_callback(handle)
        quit_editor()


try:
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).editor_request_begin_play()
    handle = unreal.register_slate_post_tick_callback(quit_when_finished)
except Exception as error:
    fail(f"{type(error).__name__}: {error}")
