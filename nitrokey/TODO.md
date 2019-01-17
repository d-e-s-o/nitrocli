- Add support for the currently unsupported commands:
    - `NK_is_AES_supported`
    - `NK_send_startup`
    - `NK_fill_SD_card_with_random_data`
    - `NK_get_SD_usage_data_as_string`
    - `NK_get_progress_bar_value`
    - `NK_list_devices_by_cpuID`
    - `NK_connect_with_ID`
- Fix timing issues with the `totp_no_pin` and `totp_pin` test cases.
- Clear passwords from memory.
- Find a nicer syntax for the `write_config` test.
- Prevent construction of internal types.
- More specific error checking in the tests.
- Check integer conversions.
- Consider implementing `Into<CommandError>` for `(Device, CommandError)`
- Lock password safe in `PasswordSafe::drop()` (see [nitrokey-storage-firmware
  issue 65][]).
- Disable creation of multiple password safes at the same time.
- Check timing in Storage tests.
- Consider restructuring `device::StorageStatus`.

[nitrokey-storage-firmware issue 65]: https://github.com/Nitrokey/nitrokey-storage-firmware/issues/65
