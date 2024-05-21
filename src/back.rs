for (index, byte) in complete_buffer.iter().enumerate() {
                if !found_content_type {
                    if index + 1 < complete_buffer.len() {
                        if complete_buffer[index] == 13 && complete_buffer[index + 1] == 10 {
                            if complete_buffer[index + 2] == 13 && complete_buffer[index + 3] == 10
                            {
                                found_content_type = true;
                            }
                        }
                    }
                } else {
                    if !found_boundary {
                        if index + 1 < complete_buffer.len() {
                            if complete_buffer[index] == 45 && complete_buffer[index + 1] == 45 {
                                if index + boundary.len() + 2 < complete_buffer.len() {
                                    if &complete_buffer[index + 2..index + 2 + boundary.len()]
                                        == boundary.as_bytes()
                                    {
                                        if index + 2 + boundary.len() + 2 < complete_buffer.len() {
                                            if complete_buffer[index + 2 + boundary.len()] == 45
                                                && complete_buffer[index + 2 + boundary.len() + 1]
                                                    == 45
                                            {
                                                found_boundary = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if found_content_type && !found_boundary {
                    part_data.push(*byte);
                }

                if found_content_type && found_boundary {
                    break;
                }
            }
