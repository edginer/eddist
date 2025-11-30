import {
  Button,
  Label,
  Modal,
  ModalBody,
  ModalHeader,
  TextInput,
} from "flowbite-react";
import React from "react";
import { useForm } from "react-hook-form";
import { toast } from "react-toastify";
import { createCap } from "~/hooks/queries";

interface CreateCapModalProps {
  open: boolean;
  setOpen: (open: boolean) => void;
  refetch: () => Promise<unknown>;
}

const CreateCapModal = ({ setOpen, refetch, open }: CreateCapModalProps) => {
  const { register, handleSubmit, reset } = useForm();

  return (
    <div>
      <Modal
        show={open}
        onClose={() => {
          setOpen(false);
          reset();
        }}
      >
        <ModalHeader>Create Cap</ModalHeader>
        <ModalBody>
          <form
            onSubmit={handleSubmit(async (data) => {
              try {
                const { mutate } = createCap({
                  body: {
                    name: data.name,
                    description: data.description,
                    password: data.password,
                  },
                });
                await mutate();
                setOpen(false);
                toast.success("Successfully created cap");
                await refetch();
              } catch (error) {
                toast.error("Failed to create cap");
              }
            })}
          >
            <div className="flex flex-col">
              <Label>Name</Label>
              <TextInput
                placeholder="Name..."
                required
                {...register("name", {
                  required: true,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Description</Label>
              <TextInput
                placeholder="Description..."
                required
                {...register("description", {
                  required: false,
                })}
              />
            </div>
            <div className="flex flex-col mt-4">
              <Label>Password</Label>
              <TextInput
                placeholder="Password..."
                required
                type="password"
                {...register("password", {
                  required: true,
                })}
              />
            </div>
            <Button type="submit" className="mt-4">
              Create
            </Button>
          </form>
        </ModalBody>
      </Modal>
    </div>
  );
};

export default CreateCapModal;
